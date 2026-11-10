use twilight_model::{application::interaction::modal::ModalInteractionData, channel::message::AllowedMentions};

use super::{
    Bot,
    event_lifecycle::post_new_event,
    event_request::{GONE, moderated_forum_of},
    event_request_messages::show_event_request,
};
use crate::{
    db::RequestMatch,
    discord::{
        answer::Answer,
        errors::{CANNOT_SEND_MESSAGES_TO_THIS_USER, is_discord_error},
        modal::{TextField, modal, submitted_value},
    },
    event::{
        document::event_name_of,
        request_view::{EventRequestAction, EventRequestOutcome, changes_asked_message, event_request_custom_id},
    },
    records::{EventRequest, RequestState},
};

const DECLINE_REASON_MAX: u16 = 300;
const REPLY_MAX: u16 = 1500;
const HOST_DMS_CLOSED: &str = "Their DMs are closed, so I cannot send them anything.";

fn moderators_only(request: &EventRequest) -> String {
    format!("Only the moderators of <#{}> can do this.", request.forum_id)
}

fn not_waiting(request: Option<EventRequest>) -> String {
    match request {
        Some(request) if request.state == RequestState::Declined => match request.handled_by {
            Some(user_id) => format!("<@{user_id}> already denied it."),
            None => "<@> already denied it.".to_owned(),
        },
        _ => GONE.to_owned(),
    }
}

pub async fn post_event_request(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<()> {
    answer.defer_update().await?;
    let (Some(request), Some(user_id)) = (bot.db.event_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    let Some(forum) = moderated_forum_of(bot, &request, user_id).await else {
        return answer.tell(moderators_only(&request)).await;
    };
    let Some(claimed) = bot.db.forget_event_request(id, RequestMatch::OnlyIfWaiting)? else {
        return answer.tell(not_waiting(bot.db.event_request_of(id)?)).await;
    };
    let thread_id = match post_new_event(bot, &forum, vec![claimed.host_id], &claimed.document).await {
        Ok(thread_id) => thread_id,
        Err(error) => {
            bot.db.save_event_request(&claimed)?;
            return Err(error);
        }
    };
    tracing::info!(request = %id, thread = %thread_id, by = %user_id, "Event request posted");
    show_event_request(bot, answer, &claimed, EventRequestOutcome::Posted { by: user_id, thread_id }).await
}

pub async fn ask_decline_reason(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<()> {
    let (Some(request), Some(user_id)) = (bot.db.event_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    if moderated_forum_of(bot, &request, user_id).await.is_none() {
        return answer.tell(moderators_only(&request)).await;
    }
    if request.state != RequestState::Waiting {
        return answer.tell(not_waiting(Some(request))).await;
    }
    let reason = TextField {
        custom_id: "reason",
        label: "Why? The host sees this",
        max_length: DECLINE_REASON_MAX,
        required: false,
        paragraph: true,
        placeholder: None,
        value: None,
    };
    let custom_id = event_request_custom_id(EventRequestAction::DeclineReason, id);
    answer.show_modal(modal(custom_id, "Deny", vec![reason])).await
}

pub async fn decline_event_request(bot: &Bot, answer: &Answer<'_>, id: &str, data: &ModalInteractionData) -> anyhow::Result<()> {
    answer.defer_update().await?;
    let (Some(request), Some(user_id)) = (bot.db.event_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    if moderated_forum_of(bot, &request, user_id).await.is_none() {
        return answer.tell(moderators_only(&request)).await;
    }
    let reason = submitted_value(data, "reason");
    let Some(declined) = bot.db.decline_event_request(id, user_id, reason.as_deref())? else {
        return answer.tell(not_waiting(bot.db.event_request_of(id)?)).await;
    };
    tracing::info!(request = %id, by = %user_id, "Event request declined");
    show_event_request(bot, answer, &declined, EventRequestOutcome::Open).await
}

pub async fn ask_reply_message(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<()> {
    let (Some(request), Some(user_id)) = (bot.db.event_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    if moderated_forum_of(bot, &request, user_id).await.is_none() {
        return answer.tell(moderators_only(&request)).await;
    }
    if request.host_dm.is_none() {
        return answer.tell(HOST_DMS_CLOSED).await;
    }
    let message = TextField {
        custom_id: "message",
        label: "Message to the host",
        max_length: REPLY_MAX,
        required: true,
        paragraph: true,
        placeholder: None,
        value: None,
    };
    let custom_id = event_request_custom_id(EventRequestAction::ReplyMessage, id);
    answer.show_modal(modal(custom_id, "Reply", vec![message])).await
}

pub async fn send_reply_to_host(bot: &Bot, answer: &Answer<'_>, id: &str, data: &ModalInteractionData) -> anyhow::Result<()> {
    answer.defer_update().await?;
    let (Some(request), Some(user_id)) = (bot.db.event_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    if moderated_forum_of(bot, &request, user_id).await.is_none() {
        return answer.tell(moderators_only(&request)).await;
    }
    let Some(message) = submitted_value(data, "message") else {
        return Ok(());
    };
    let Some(dm) = request.host_dm else {
        return answer.tell(HOST_DMS_CLOSED).await;
    };
    let guild_name = bot.guild_name(request.guild_id);
    let content = changes_asked_message(&event_name_of(&request.document), user_id, &guild_name, &message);
    let sent = bot
        .http
        .create_message(dm.channel_id)
        .content(&content)
        .reply(dm.message_id)
        .fail_if_not_exists(false)
        .allowed_mentions(Some(&AllowedMentions::default()))
        .await
        .map_err(anyhow::Error::from);
    match sent {
        Ok(_) => {
            tracing::info!(request = %id, by = %user_id, "Event request reply sent");
            answer.tell(format!("Sent to <@{}>.", request.host_id)).await
        }
        Err(error) if is_discord_error(&error, CANNOT_SEND_MESSAGES_TO_THIS_USER) => answer.tell(HOST_DMS_CLOSED).await,
        Err(error) => Err(error),
    }
}

pub async fn withdraw_event_request(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<()> {
    answer.defer_update().await?;
    let request = bot.db.event_request_of(id)?;
    if request.as_ref().is_some_and(|request| Some(request.host_id) != answer.interaction.author_id()) {
        return answer.tell("Only the host can withdraw it.").await;
    }
    let withdrawn = match request {
        Some(_) => bot.db.forget_event_request(id, RequestMatch::AnyState)?,
        None => None,
    };
    let Some(withdrawn) = withdrawn else {
        return answer.tell(GONE).await;
    };
    tracing::info!(request = %id, "Event request withdrawn");
    show_event_request(bot, answer, &withdrawn, EventRequestOutcome::Withdrawn).await
}
