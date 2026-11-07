use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use twilight_model::id::Id;

use super::{Answered, JSON_MEDIA_TYPE, OPEN_EVNT_MEDIA_TYPE, answer_unless_unchanged, not_found};
use crate::{
    bot::Bot,
    event::{
        document::text_of,
        index::{ListedEvent, event_document_body, open_evnt_index_of},
    },
    records::EventPost,
};

fn id_of_file<T>(file: &str, extension: &str) -> Option<Id<T>> {
    let name = file.strip_suffix(extension)?;
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    name.parse().ok().and_then(Id::new_checked)
}

fn is_token(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn answer_index(headers: &HeaderMap, events: &[EventPost], href_of: impl Fn(&EventPost) -> String) -> Response {
    let listed = events
        .iter()
        .map(|event| ListedEvent { event, href: href_of(event), name: text_of(event.document.get("name")) })
        .collect();
    answer_unless_unchanged(headers, open_evnt_index_of(listed).to_string(), JSON_MEDIA_TYPE)
}

fn answer_event_document(headers: &HeaderMap, event: &EventPost) -> Response {
    answer_unless_unchanged(headers, event_document_body(event), OPEN_EVNT_MEDIA_TYPE)
}

pub async fn event_document(State(bot): State<Arc<Bot>>, Path(file): Path<String>, headers: HeaderMap) -> Answered {
    let Some(thread_id) = id_of_file(&file, ".evnt.json") else {
        return Ok(not_found());
    };
    let Some(event) = bot.db.event_of_thread(thread_id)? else {
        return Ok(not_found());
    };
    if !bot.everyone_can_view_forum(event.guild_id, event.forum_id).await {
        return Ok(not_found());
    }
    Ok(answer_event_document(&headers, &event))
}

pub async fn forum_index(State(bot): State<Arc<Bot>>, Path(file): Path<String>, headers: HeaderMap) -> Answered {
    let Some(forum_id) = id_of_file(&file, ".json") else {
        return Ok(not_found());
    };
    let Some(forum) = bot.db.event_forum_of(forum_id)? else {
        return Ok(not_found());
    };
    if !bot.everyone_can_view_forum(forum.guild_id, forum.forum_id).await {
        return Ok(not_found());
    }
    let events = bot.db.events_of_forum(forum.forum_id)?;
    Ok(answer_index(&headers, &events, |event| bot.links.event_document_url(event.thread_id)))
}

pub async fn calendar_index(State(bot): State<Arc<Bot>>, Path(file): Path<String>, headers: HeaderMap) -> Answered {
    let Some(token) = file.strip_suffix(".json").filter(|token| is_token(token)) else {
        return Ok(not_found());
    };
    let Some(link) = bot.db.calendar_link_of_token(token)? else {
        return Ok(not_found());
    };
    let events = bot.events_joined_by(link.user_id).await?;
    Ok(answer_index(&headers, &events, |event| bot.links.calendar_document_url(&link.token, event.thread_id)))
}

pub async fn calendar_document(
    State(bot): State<Arc<Bot>>,
    Path((token, file)): Path<(String, String)>,
    headers: HeaderMap,
) -> Answered {
    let (true, Some(thread_id)) = (is_token(&token), id_of_file(&file, ".evnt.json")) else {
        return Ok(not_found());
    };
    let Some(link) = bot.db.calendar_link_of_token(&token)? else {
        return Ok(not_found());
    };
    let joined = bot.events_joined_by(link.user_id).await?;
    Ok(match joined.iter().find(|event| event.thread_id == thread_id) {
        Some(event) => answer_event_document(&headers, event),
        None => not_found(),
    })
}
