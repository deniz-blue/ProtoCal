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
    meet::{
        document::text_of,
        index::{ListedMeet, meet_document_body, open_evnt_index_of},
    },
    records::Meet,
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

fn answer_index(headers: &HeaderMap, meets: &[Meet], href_of: impl Fn(&Meet) -> String) -> Response {
    let listed = meets
        .iter()
        .map(|meet| ListedMeet { meet, href: href_of(meet), name: text_of(meet.document.get("name")) })
        .collect();
    answer_unless_unchanged(headers, open_evnt_index_of(listed).to_string(), JSON_MEDIA_TYPE)
}

fn answer_meet_document(headers: &HeaderMap, meet: &Meet) -> Response {
    answer_unless_unchanged(headers, meet_document_body(meet), OPEN_EVNT_MEDIA_TYPE)
}

pub async fn meet_document(State(bot): State<Arc<Bot>>, Path(file): Path<String>, headers: HeaderMap) -> Answered {
    let Some(thread_id) = id_of_file(&file, ".evnt.json") else {
        return Ok(not_found());
    };
    let Some(meet) = bot.db.meet_of_thread(thread_id)? else {
        return Ok(not_found());
    };
    if !bot.everyone_can_view_forum(meet.guild_id, meet.forum_id).await {
        return Ok(not_found());
    }
    Ok(answer_meet_document(&headers, &meet))
}

pub async fn forum_index(State(bot): State<Arc<Bot>>, Path(file): Path<String>, headers: HeaderMap) -> Answered {
    let Some(forum_id) = id_of_file(&file, ".json") else {
        return Ok(not_found());
    };
    let Some(forum) = bot.db.meet_forum_of(forum_id)? else {
        return Ok(not_found());
    };
    if !bot.everyone_can_view_forum(forum.guild_id, forum.forum_id).await {
        return Ok(not_found());
    }
    let meets = bot.db.meets_of_forum(forum.forum_id)?;
    Ok(answer_index(&headers, &meets, |meet| bot.links.meet_document_url(meet.thread_id)))
}

pub async fn calendar_index(State(bot): State<Arc<Bot>>, Path(file): Path<String>, headers: HeaderMap) -> Answered {
    let Some(token) = file.strip_suffix(".json").filter(|token| is_token(token)) else {
        return Ok(not_found());
    };
    let Some(link) = bot.db.calendar_link_of_token(token)? else {
        return Ok(not_found());
    };
    let meets = bot.meets_joined_by(link.user_id).await?;
    Ok(answer_index(&headers, &meets, |meet| bot.links.calendar_document_url(&link.token, meet.thread_id)))
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
    let joined = bot.meets_joined_by(link.user_id).await?;
    Ok(match joined.iter().find(|meet| meet.thread_id == thread_id) {
        Some(meet) => answer_meet_document(&headers, meet),
        None => not_found(),
    })
}
