use twilight_model::application::interaction::modal::ModalInteractionData;

use super::{
    Bot,
    meet_lifecycle::post_new_meet,
    meet_request::{GONE, moderated_forum_of},
    meet_request_messages::show_meet_request,
};
use crate::{
    db::RequestMatch,
    discord::{
        answer::Answer,
        modal::{TextField, modal, submitted_value},
    },
    meet::request_view::{MeetRequestAction, MeetRequestOutcome, meet_request_custom_id},
    records::{MeetRequest, RequestState},
};

const DECLINE_REASON_MAX: u16 = 300;

fn moderators_only(request: &MeetRequest) -> String {
    format!("Only the moderators of <#{}> can do this.", request.forum_id)
}

fn not_waiting(request: Option<MeetRequest>) -> String {
    match request {
        Some(request) if request.state == RequestState::Declined => match request.handled_by {
            Some(user_id) => format!("<@{user_id}> already declined it."),
            None => "<@> already declined it.".to_owned(),
        },
        _ => GONE.to_owned(),
    }
}

pub async fn post_meet_request(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<()> {
    answer.defer_update().await?;
    let (Some(request), Some(user_id)) = (bot.db.meet_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    let Some(forum) = moderated_forum_of(bot, &request, user_id).await else {
        return answer.tell(moderators_only(&request)).await;
    };
    let Some(claimed) = bot.db.forget_meet_request(id, RequestMatch::OnlyIfWaiting)? else {
        return answer.tell(not_waiting(bot.db.meet_request_of(id)?)).await;
    };
    let thread_id = match post_new_meet(bot, &forum, vec![claimed.host_id], &claimed.document).await {
        Ok(thread_id) => thread_id,
        Err(error) => {
            bot.db.save_meet_request(&claimed)?;
            return Err(error);
        }
    };
    tracing::info!(request = %id, thread = %thread_id, by = %user_id, "Meet request posted");
    show_meet_request(bot, answer, &claimed, MeetRequestOutcome::Posted { by: user_id, thread_id }).await
}

pub async fn ask_decline_reason(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<()> {
    let (Some(request), Some(user_id)) = (bot.db.meet_request_of(id)?, answer.interaction.author_id()) else {
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
    let custom_id = meet_request_custom_id(MeetRequestAction::DeclineReason, id);
    answer.show_modal(modal(custom_id, "Decline", vec![reason])).await
}

pub async fn decline_meet_request(bot: &Bot, answer: &Answer<'_>, id: &str, data: &ModalInteractionData) -> anyhow::Result<()> {
    answer.defer_update().await?;
    let (Some(request), Some(user_id)) = (bot.db.meet_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    if moderated_forum_of(bot, &request, user_id).await.is_none() {
        return answer.tell(moderators_only(&request)).await;
    }
    let reason = submitted_value(data, "reason");
    let Some(declined) = bot.db.decline_meet_request(id, user_id, reason.as_deref())? else {
        return answer.tell(not_waiting(bot.db.meet_request_of(id)?)).await;
    };
    tracing::info!(request = %id, by = %user_id, "Meet request declined");
    show_meet_request(bot, answer, &declined, MeetRequestOutcome::Open).await
}

pub async fn withdraw_meet_request(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<()> {
    answer.defer_update().await?;
    let request = bot.db.meet_request_of(id)?;
    if request.as_ref().is_some_and(|request| Some(request.host_id) != answer.interaction.author_id()) {
        return answer.tell("Only the host can withdraw it.").await;
    }
    let withdrawn = match request {
        Some(_) => bot.db.forget_meet_request(id, RequestMatch::AnyState)?,
        None => None,
    };
    let Some(withdrawn) = withdrawn else {
        return answer.tell(GONE).await;
    };
    tracing::info!(request = %id, "Meet request withdrawn");
    show_meet_request(bot, answer, &withdrawn, MeetRequestOutcome::Withdrawn).await
}
