use serde_json::Value;
use twilight_model::{
    channel::message::{
        Component,
        component::{ButtonStyle, TextDisplay},
    },
    id::{
        Id,
        marker::{ChannelMarker, UserMarker},
    },
};

use super::{
    fields::fields_of_event_document,
    post::{MESSAGE_MAX, EventFormTarget, event_post_content},
};
use crate::{
    components::{COMPONENTS_MAX, button, component_count, container, row, text_display, text_displays},
    records::{EventRequest, RequestState},
    text::utf16_length,
};

// Buttons already posted to Discord carry this id.
const EVENT_REQUEST_BUTTON: &str = "meet-request";
const EXPIRED: &str = "⌛ Nobody got to it before the event would have been over.";
const CLOSED_DMS: &str = "-# Your DMs are closed, so this is the only place you can follow it.";
const HOST_REQUESTS_HEAD: &str = "Your DMs were closed when you sent these, so they wait here:";
const NO_HOST_REQUESTS: &str = "Nothing of yours is waiting.";
const HOST_REQUESTS_HEAD_ID: i32 = 9001;
pub const TEXT_DISPLAYS_MAX: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRequestAction {
    Post,
    Decline,
    DeclineReason,
    Reply,
    ReplyMessage,
    Withdraw,
}

impl EventRequestAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Post => "post",
            Self::Decline => "decline",
            Self::DeclineReason => "decline-reason",
            Self::Reply => "reply",
            Self::ReplyMessage => "reply-message",
            Self::Withdraw => "withdraw",
        }
    }

    fn of_str(text: &str) -> Option<Self> {
        [Self::Post, Self::Decline, Self::DeclineReason, Self::Reply, Self::ReplyMessage, Self::Withdraw]
            .into_iter()
            .find(|action| action.as_str() == text)
    }
}

pub fn event_request_custom_id(action: EventRequestAction, id: &str) -> String {
    format!("{EVENT_REQUEST_BUTTON}:{}:{id}", action.as_str())
}

pub fn event_request_action_of(custom_id: &str) -> Option<(EventRequestAction, String)> {
    let mut parts = custom_id.split(':');
    if parts.next()? != EVENT_REQUEST_BUTTON {
        return None;
    }
    let action = EventRequestAction::of_str(parts.next()?)?;
    let id = parts.next().filter(|id| !id.is_empty())?;
    Some((action, id.to_owned()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRequestOutcome {
    Open,
    Posted {
        by: Id<UserMarker>,
        thread_id: Id<ChannelMarker>,
    },
    Withdrawn,
    Expired,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageView {
    pub components: Vec<Component>,
}

fn edit_button(request: &EventRequest, label: &str) -> Component {
    let target = EventFormTarget::Request(request.id.clone());
    button(target.custom_id(), label, ButtonStyle::Secondary)
}

fn with_preview(request: &EventRequest, head: &[String], buttons: Vec<Component>) -> MessageView {
    let lines: Vec<&str> = head.iter().map(String::as_str).filter(|line| !line.is_empty()).collect();
    let top = lines.join("\n");
    let room = MESSAGE_MAX.min(TEXT_DISPLAYS_MAX.saturating_sub(utf16_length(&top)));
    let preview = event_post_content(&request.document, &[request.host_id], request.updated_at, room);
    let mut components = vec![container(vec![text_display(preview)])];
    if !top.is_empty() {
        components.push(text_display(top));
    }
    if !buttons.is_empty() {
        components.push(row(buttons));
    }
    MessageView { components }
}

fn reason_of(request: &EventRequest) -> String {
    request
        .decline_reason
        .as_ref()
        .filter(|reason| !reason.is_empty())
        .map_or_else(String::new, |reason| format!(": {reason}"))
}

pub fn event_request_card(request: &EventRequest, outcome: EventRequestOutcome) -> MessageView {
    let mut head = vec![
        format!("<@{}> wants to post this in <#{}>", request.host_id, request.forum_id),
        if request.host_edited { "-# Edited by the host".to_owned() } else { String::new() },
        if outcome == EventRequestOutcome::Open && request.host_dm.is_none() {
            "-# Their DMs are closed, so I cannot tell them what happens.".to_owned()
        } else {
            String::new()
        },
    ];
    let status = match outcome {
        EventRequestOutcome::Posted { by, thread_id } => format!("✅ Approved by <@{by}>: <#{thread_id}>"),
        EventRequestOutcome::Withdrawn => "↩️ Withdrawn by the host".to_owned(),
        EventRequestOutcome::Expired => EXPIRED.to_owned(),
        EventRequestOutcome::Open if request.state == RequestState::Declined => {
            let by = request.handled_by.map_or_else(String::new, |id| id.to_string());
            format!("❌ Denied by <@{by}>{}", reason_of(request))
        }
        EventRequestOutcome::Open => {
            let buttons = vec![
                button(event_request_custom_id(EventRequestAction::Post, &request.id), "Approve", ButtonStyle::Primary),
                edit_button(request, "Edit"),
                button(event_request_custom_id(EventRequestAction::Reply, &request.id), "Reply", ButtonStyle::Secondary),
                button(event_request_custom_id(EventRequestAction::Decline, &request.id), "Deny", ButtonStyle::Danger),
            ];
            return with_preview(request, &head, buttons);
        }
    };
    head.push(status);
    with_preview(request, &head, Vec::new())
}

pub fn changes_asked_message(event_name: &str, moderator_id: Id<UserMarker>, guild_name: &str, message: &str) -> String {
    format!("-# ✏️ Changes asked for **{event_name}** by <@{moderator_id}> at **{guild_name}**\n{message}")
}

fn host_status(request: &EventRequest, outcome: EventRequestOutcome, guild_name: &str) -> (String, Vec<Component>) {
    let moderators = format!("the moderators of <#{}> in **{guild_name}**", request.forum_id);
    let withdraw = || button(event_request_custom_id(EventRequestAction::Withdraw, &request.id), "Withdraw", ButtonStyle::Secondary);
    match outcome {
        EventRequestOutcome::Posted { thread_id, .. } => {
            let changes = if request.moderators_edited { ", with changes by the moderators" } else { "" };
            (format!("✅ Posted: <#{thread_id}>{changes}"), Vec::new())
        }
        EventRequestOutcome::Withdrawn => ("↩️ You withdrew it.".to_owned(), Vec::new()),
        EventRequestOutcome::Expired => (EXPIRED.to_owned(), Vec::new()),
        EventRequestOutcome::Open if request.state == RequestState::Declined => (
            format!("❌ Denied by {moderators}{}", reason_of(request)),
            vec![edit_button(request, "Edit and send again"), withdraw()],
        ),
        EventRequestOutcome::Open => (format!("Waiting for {moderators}."), vec![edit_button(request, "Edit"), withdraw()]),
    }
}

pub fn event_request_host_view(request: &EventRequest, outcome: EventRequestOutcome, guild_name: &str) -> MessageView {
    let (status, buttons) = host_status(request, outcome, guild_name);
    let only_followed_here = outcome == EventRequestOutcome::Open
        && request.state == RequestState::Waiting
        && request.host_dm.is_none();
    let closed_dms = if only_followed_here { CLOSED_DMS.to_owned() } else { String::new() };
    with_preview(request, &[status, closed_dms], buttons)
}

fn fits_in_one_message(components: &[Component]) -> bool {
    component_count(components) <= COMPONENTS_MAX
        && text_displays(components).iter().map(|text| utf16_length(text)).sum::<usize>() <= TEXT_DISPLAYS_MAX
}

fn event_summary(document: &Value) -> String {
    let fields = fields_of_event_document(document);
    match fields.date {
        Some(date) => format!("**{}**\n{date}", fields.name),
        None => format!("**{}**", fields.name),
    }
}

pub fn is_host_requests_list(components: &[Component]) -> bool {
    matches!(components.first(), Some(Component::TextDisplay(head)) if head.id == Some(HOST_REQUESTS_HEAD_ID))
}

pub fn host_requests_view(requests: &[EventRequest], guild_name: &str, new_event: Vec<Component>) -> MessageView {
    let head = if requests.is_empty() { NO_HOST_REQUESTS } else { HOST_REQUESTS_HEAD };
    let mut components = vec![Component::TextDisplay(TextDisplay { id: Some(HOST_REQUESTS_HEAD_ID), content: head.to_owned() })];
    for request in requests {
        let (status, buttons) = host_status(request, EventRequestOutcome::Open, guild_name);
        let entry = [container(vec![text_display(event_summary(&request.document))]), text_display(status), row(buttons)];
        if !fits_in_one_message(&[components.as_slice(), &entry, &new_event].concat()) {
            break;
        }
        components.extend(entry);
    }
    components.extend(new_event);
    MessageView { components }
}
