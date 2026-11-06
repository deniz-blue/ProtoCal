use twilight_model::application::interaction::{
    Interaction, InteractionData, message_component::MessageComponentInteractionData, modal::ModalInteractionData,
};

use super::{
    Bot,
    commands::run_command,
    meet_edit::answer_edit_button,
    meet_form::{open_meet_form, open_picked_meet_form, submit_meet_form},
    meet_request_moderation::{ask_decline_reason, decline_meet_request, post_meet_request, withdraw_meet_request},
};
use crate::{
    discord::answer::Answer,
    meet::{
        post::{MEET_FORUM_PICK, MeetFormTarget, thread_of_edit_button},
        request_view::{MeetRequestAction, meet_request_action_of},
    },
};

pub async fn route(bot: &Bot, interaction: &Interaction) -> anyhow::Result<()> {
    tracing::debug!(id = %interaction.id, kind = ?interaction.kind, "Interaction received");
    let answer = Answer::new(&bot.http, bot.application_id, interaction);
    let handled = match &interaction.data {
        Some(InteractionData::ApplicationCommand(data)) => run_command(bot, &answer, data).await,
        Some(InteractionData::MessageComponent(data)) => press_component(bot, &answer, data).await,
        Some(InteractionData::ModalSubmit(data)) => submit_modal(bot, &answer, data).await,
        _ => Ok(()),
    };
    if let Err(error) = &handled {
        tracing::error!(?error, "Event handler failed");
        if let Err(answer_error) = answer.answer_failure().await {
            tracing::error!(error = ?answer_error, "Failed interaction was not answered");
        }
    }
    Ok(())
}

async fn press_component(bot: &Bot, answer: &Answer<'_>, data: &MessageComponentInteractionData) -> anyhow::Result<()> {
    let custom_id = data.custom_id.as_str();
    if custom_id == MEET_FORUM_PICK {
        return open_picked_meet_form(bot, answer, data.values.first().map(String::as_str)).await;
    }
    if let Some(thread_id) = thread_of_edit_button(custom_id) {
        return answer_edit_button(bot, answer, thread_id).await;
    }
    if let Some(target) = MeetFormTarget::of_custom_id(custom_id) {
        return open_meet_form(bot, answer, &target).await;
    }
    match meet_request_action_of(custom_id) {
        Some((MeetRequestAction::Post, id)) => post_meet_request(bot, answer, &id).await,
        Some((MeetRequestAction::Decline, id)) => ask_decline_reason(bot, answer, &id).await,
        Some((MeetRequestAction::Withdraw, id)) => withdraw_meet_request(bot, answer, &id).await,
        Some((MeetRequestAction::DeclineReason, _)) | None => Ok(()),
    }
}

async fn submit_modal(bot: &Bot, answer: &Answer<'_>, data: &ModalInteractionData) -> anyhow::Result<()> {
    if let Some(target) = MeetFormTarget::of_custom_id(&data.custom_id) {
        return submit_meet_form(bot, answer, target, data).await;
    }
    match meet_request_action_of(&data.custom_id) {
        Some((MeetRequestAction::DeclineReason, id)) => decline_meet_request(bot, answer, &id, data).await,
        _ => Ok(()),
    }
}
