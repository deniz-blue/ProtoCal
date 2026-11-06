use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use super::{Answered, edit_return_page::edit_return_page, html_answer, text_answer};
use crate::{bot::Bot, meet::document::validate_meet_document};

const EDIT_LINK_EXPIRED: &str = "This edit link has run out. Ask for a new one in Discord.";

pub async fn show_edit_return_page() -> Response {
    html_answer(edit_return_page())
}

fn edit_answer(status: StatusCode, message: &str, thread_url: Option<&str>) -> Answered {
    let body = match thread_url {
        Some(thread_url) => json!({ "message": message, "threadUrl": thread_url }),
        None => json!({ "message": message }),
    };
    Ok((status, Json(body)).into_response())
}

fn parsed_event(event: Option<&Value>) -> Option<Value> {
    match event? {
        Value::String(text) => serde_json::from_str(text).ok(),
        other => Some(other.clone()),
    }
}

pub async fn save_edit_return(State(bot): State<Arc<Bot>>, body: Bytes) -> Answered {
    let sent: Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
    let token = sent.get("state").and_then(Value::as_str).unwrap_or("");
    let Some(grant) = bot.edit_grants.find(token) else {
        return edit_answer(StatusCode::FORBIDDEN, EDIT_LINK_EXPIRED, None);
    };
    let meet = bot.db.meet_of_thread(grant.thread_id)?;
    let thread = bot.channel(grant.thread_id).await.filter(|thread| thread.is_thread());
    let (Some(meet), Some(thread)) = (meet, thread) else {
        return edit_answer(StatusCode::BAD_REQUEST, "That meet is over or its post is gone.", None);
    };
    let thread_url = format!("https://discord.com/channels/{}/{}", thread.guild_id, thread.id);
    let thread_url = Some(thread_url.as_str());
    if sent.get("cancelled") == Some(&Value::Bool(true)) {
        return edit_answer(StatusCode::OK, "Nothing was changed.", thread_url);
    }
    let event = parsed_event(sent.get("event")).filter(|event| {
        event.as_object().is_some_and(|fields| fields.get("id") == meet.document.get("id"))
    });
    let Some(event) = event else {
        return edit_answer(StatusCode::BAD_REQUEST, "That is not this meet's event. Its id does not match.", thread_url);
    };
    if let Err(problem) = validate_meet_document(&event) {
        return edit_answer(StatusCode::BAD_REQUEST, &problem.message, thread_url);
    }
    crate::bot::meet_lifecycle::save_meet_document(&bot, &thread, &meet, event).await?;
    bot.edit_grants.spend(token);
    edit_answer(StatusCode::OK, "Saved.", thread_url)
}

pub async fn open_editor(State(bot): State<Arc<Bot>>, Path(token): Path<String>) -> Answered {
    let grant = bot.edit_grants.find(&token);
    let meet = match grant {
        Some(grant) => bot.db.meet_of_thread(grant.thread_id)?,
        None => None,
    };
    let Some(meet) = meet else {
        return Ok(text_answer(StatusCode::FORBIDDEN, EDIT_LINK_EXPIRED));
    };
    let editor_url = bot.links.editor_url(&meet.document.to_string(), &token);
    Ok((StatusCode::FOUND, [(header::LOCATION, editor_url)]).into_response())
}
