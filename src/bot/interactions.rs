use twilight_model::application::interaction::{
    Interaction, InteractionData, message_component::MessageComponentInteractionData, modal::ModalInteractionData,
};

use super::{
    Bot,
    commands::run_command,
    event_edit::answer_edit_button,
    event_form::{offer_new_event_form, open_event_form, open_picked_event_form, submit_event_form},
    event_request_moderation::{
        ask_decline_reason, ask_reply_message, decline_event_request, post_event_request, send_reply_to_host,
        withdraw_event_request,
    },
};
use crate::{
    discord::answer::Answer,
    event::{
        post::{EVENT_FORUM_PICK, EventFormTarget, NewEventButton, thread_of_edit_button},
        request_view::{EventRequestAction, event_request_action_of},
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
    if custom_id == EVENT_FORUM_PICK {
        return open_picked_event_form(bot, answer, data.values.first().map(String::as_str)).await;
    }
    if let Some(pressed) = NewEventButton::of_custom_id(custom_id) {
        return offer_new_event_form(bot, answer, pressed.forum_id).await;
    }
    if let Some(thread_id) = thread_of_edit_button(custom_id) {
        return answer_edit_button(bot, answer, thread_id).await;
    }
    if let Some(target) = EventFormTarget::of_custom_id(custom_id) {
        return open_event_form(bot, answer, &target).await;
    }
    match event_request_action_of(custom_id) {
        Some((EventRequestAction::Post, id)) => post_event_request(bot, answer, &id).await,
        Some((EventRequestAction::Decline, id)) => ask_decline_reason(bot, answer, &id).await,
        Some((EventRequestAction::Reply, id)) => ask_reply_message(bot, answer, &id).await,
        Some((EventRequestAction::Withdraw, id)) => withdraw_event_request(bot, answer, &id).await,
        Some((EventRequestAction::DeclineReason | EventRequestAction::ReplyMessage, _)) | None => Ok(()),
    }
}

async fn submit_modal(bot: &Bot, answer: &Answer<'_>, data: &ModalInteractionData) -> anyhow::Result<()> {
    if let Some(target) = EventFormTarget::of_custom_id(&data.custom_id) {
        return submit_event_form(bot, answer, target, data).await;
    }
    match event_request_action_of(&data.custom_id) {
        Some((EventRequestAction::DeclineReason, id)) => decline_event_request(bot, answer, &id, data).await,
        Some((EventRequestAction::ReplyMessage, id)) => send_reply_to_host(bot, answer, &id, data).await,
        _ => Ok(()),
    }
}
