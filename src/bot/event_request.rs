use serde_json::Value;
use twilight_model::{
    channel::ChannelType,
    id::{
        Id,
        marker::UserMarker,
    },
};

use super::{
    Bot,
    access::OpenEventForum,
    event_form_modal::reject_draft,
    event_form_submit::SubmittedForm,
    event_request_messages::{edit_card, edit_host_dm, post_view, show_event_request, show_host_edited_event_request},
};
use crate::{
    db::RequestEditor,
    discord::{
        answer::{Answer, Reply},
        cache::CachedChannel,
        errors::is_direct_message_refused,
    },
    event::{
        dates::event_is_over_at,
        fields::event_document_of_fields,
        request_view::{EventRequestOutcome, event_request_card, event_request_host_view},
        written_date::today_in_event_time_zone,
    },
    records::{EventRequest, MessageAt, RequestState},
    time::now_unix_seconds,
};

pub const GONE: &str = "This request is gone: it was posted, withdrawn or ran out.";
const REVIEW_CHANNEL_UNREACHABLE: &str = "I could not post in this forum's review channel. Please tell staff.";
const EDITORS_ONLY: &str = "Only the host or the moderators can edit this.";

const fn can_hold_messages(channel: &CachedChannel) -> bool {
    !matches!(
        channel.kind,
        ChannelType::GuildCategory | ChannelType::GuildForum | ChannelType::GuildMedia | ChannelType::GuildDirectory
    )
}

async fn send_host_dm(bot: &Bot, request: &EventRequest, guild_name: &str) -> anyhow::Result<Option<MessageAt>> {
    let channel = bot.http.create_private_channel(request.host_id).await?.model().await?;
    let with_dm = EventRequest { host_dm: Some(MessageAt { channel_id: channel.id, message_id: Id::new(1) }), ..request.clone() };
    let part_of = bot.part_of(&request.document).await;
    let view = event_request_host_view(&with_dm, part_of.as_ref(), EventRequestOutcome::Open, guild_name);
    match post_view(bot, channel.id, &view).await {
        Ok(at) => Ok(Some(at)),
        Err(error) if is_direct_message_refused(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub async fn send_event_request(
    bot: &Bot,
    answer: &Answer<'_>,
    forum: &OpenEventForum,
    document: Value,
    form: SubmittedForm,
) -> anyhow::Result<()> {
    let review_channel = match forum.settings.review_channel_id {
        Some(channel_id) => bot.channel(channel_id).await.filter(can_hold_messages),
        None => None,
    };
    let (Some(review_channel), Some(host_id)) = (review_channel, answer.interaction.author_id()) else {
        return reject_draft(bot, answer, &form.target, form.fields, REVIEW_CHANNEL_UNREACHABLE).await;
    };
    let id = uuid::Uuid::new_v4().to_string();
    let mut document = document;
    document["id"] = Value::String(format!("urn:uuid:{id}"));
    let request = EventRequest {
        id: id.clone(),
        guild_id: forum.channel.guild_id,
        forum_id: forum.channel.id,
        host_id,
        document,
        state: RequestState::Waiting,
        handled_by: None,
        decline_reason: None,
        host_edited: false,
        moderators_edited: false,
        card_channel_id: review_channel.id,
        card_message_id: None,
        host_dm: None,
        updated_at: now_unix_seconds(),
    };
    let guild_name = bot.guild_name(forum.channel.guild_id);
    let sent = EventRequest { host_dm: send_host_dm(bot, &request, &guild_name).await?, ..request };
    let part_of = bot.part_of(&sent.document).await;
    let card = match post_view(bot, review_channel.id, &event_request_card(&sent, part_of.as_ref(), EventRequestOutcome::Open)).await {
        Ok(card) => card,
        Err(error) => {
            tracing::warn!(forum = %forum.channel.id, ?error, "Event request card was not posted");
            if let Some(dm) = sent.host_dm {
                let _ = bot.http.delete_message(dm.channel_id, dm.message_id).await;
            }
            return reject_draft(bot, answer, &form.target, form.fields, REVIEW_CHANNEL_UNREACHABLE).await;
        }
    };
    let saved = EventRequest { card_message_id: Some(card.message_id), ..sent };
    bot.db.save_event_request(&saved)?;
    bot.forget_rejected_draft(host_id, &form.target.key());
    tracing::info!(request = %id, forum = %forum.channel.id, "Event request sent");
    let reply = match saved.host_dm {
        Some(_) => Reply::text(format!("Sent to the moderators of <#{}>.", forum.channel.id)),
        None => event_request_host_view(&saved, part_of.as_ref(), EventRequestOutcome::Open, &guild_name).into(),
    };
    answer.edit_reply(reply).await
}

pub async fn moderated_forum_of(bot: &Bot, request: &EventRequest, user_id: Id<UserMarker>) -> Option<CachedChannel> {
    let forum = bot.forum_of_guild(request.guild_id, request.forum_id).await?;
    let member = bot.member_roles(request.guild_id, user_id).await?;
    bot.is_event_moderator(&member, &forum).then_some(forum)
}

async fn editor_of_event_request(bot: &Bot, request: &EventRequest, user_id: Id<UserMarker>) -> Option<RequestEditor> {
    if request.host_id == user_id {
        return Some(RequestEditor::Host);
    }
    moderated_forum_of(bot, request, user_id).await.map(|_| RequestEditor::Moderator)
}

pub async fn event_request_to_edit(bot: &Bot, answer: &Answer<'_>, id: &str) -> anyhow::Result<Option<EventRequest>> {
    let Some(request) = bot.db.event_request_of(id)? else {
        answer.tell(GONE).await?;
        return Ok(None);
    };
    let Some(user_id) = answer.interaction.author_id() else {
        return Ok(None);
    };
    if editor_of_event_request(bot, &request, user_id).await.is_none() {
        answer.tell(EDITORS_ONLY).await?;
        return Ok(None);
    }
    Ok(Some(request))
}

pub async fn save_event_request_edit(bot: &Bot, answer: &Answer<'_>, id: &str, form: SubmittedForm) -> anyhow::Result<()> {
    let (Some(request), Some(user_id)) = (bot.db.event_request_of(id)?, answer.interaction.author_id()) else {
        return answer.tell(GONE).await;
    };
    let Some(editor) = editor_of_event_request(bot, &request, user_id).await else {
        return answer.tell(EDITORS_ONLY).await;
    };
    let document = match event_document_of_fields(&form.fields, &request.document, today_in_event_time_zone()) {
        Ok(document) => document,
        Err(problem) => return reject_draft(bot, answer, &form.target, form.fields, &problem.message).await,
    };
    answer.defer_update().await?;
    bot.forget_rejected_draft(user_id, &form.target.key());
    let Some(edited) = bot.db.edit_event_request(id, &document, editor, now_unix_seconds())? else {
        return answer.tell(GONE).await;
    };
    let sends_again = editor == RequestEditor::Host && request.state == RequestState::Declined;
    tracing::info!(request = %id, ?editor, sends_again, "Event request edited");
    match editor {
        RequestEditor::Host => show_host_edited_event_request(bot, answer, &edited).await,
        RequestEditor::Moderator => show_event_request(bot, answer, &edited, EventRequestOutcome::Open).await,
    }
}

pub async fn drop_event_requests_that_are_over(bot: &Bot, now_unix_seconds: i64) -> anyhow::Result<()> {
    for request in bot.db.all_event_requests()? {
        if event_is_over_at(&request.document).is_none_or(|over_at| over_at > now_unix_seconds) {
            continue;
        }
        let dropped = bot.db.forget_event_request(&request.id, crate::db::RequestMatch::AnyState)?;
        let Some(dropped) = dropped.filter(|dropped| dropped.state == RequestState::Waiting) else {
            continue;
        };
        tracing::info!(request = %request.id, "Event request ran out");
        edit_card(bot, &dropped, EventRequestOutcome::Expired).await;
        edit_host_dm(bot, &dropped, EventRequestOutcome::Expired).await;
    }
    Ok(())
}
