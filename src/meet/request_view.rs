use twilight_model::{
    channel::message::{Component, component::ButtonStyle},
    id::{
        Id,
        marker::{ChannelMarker, UserMarker},
    },
};

use super::post::{MESSAGE_MAX, MeetFormTarget, meet_post_content};
use crate::{
    components::{button, row},
    records::{MeetRequest, RequestState},
    text::utf16_length,
};

const MEET_REQUEST_BUTTON: &str = "meet-request";
const EXPIRED: &str = "⌛ Nobody got to it before the meet would have been over.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetRequestAction {
    Post,
    Decline,
    DeclineReason,
    Withdraw,
}

impl MeetRequestAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Post => "post",
            Self::Decline => "decline",
            Self::DeclineReason => "decline-reason",
            Self::Withdraw => "withdraw",
        }
    }

    fn of_str(text: &str) -> Option<Self> {
        [Self::Post, Self::Decline, Self::DeclineReason, Self::Withdraw]
            .into_iter()
            .find(|action| action.as_str() == text)
    }
}

pub fn meet_request_custom_id(action: MeetRequestAction, id: &str) -> String {
    format!("{MEET_REQUEST_BUTTON}:{}:{id}", action.as_str())
}

pub fn meet_request_action_of(custom_id: &str) -> Option<(MeetRequestAction, String)> {
    let mut parts = custom_id.split(':');
    if parts.next()? != MEET_REQUEST_BUTTON {
        return None;
    }
    let action = MeetRequestAction::of_str(parts.next()?)?;
    let id = parts.next().filter(|id| !id.is_empty())?;
    Some((action, id.to_owned()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetRequestOutcome {
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
    pub content: String,
    pub components: Vec<Component>,
}

fn edit_button(request: &MeetRequest, label: &str) -> Component {
    let target = MeetFormTarget::Request(request.id.clone());
    button(target.custom_id(), label, ButtonStyle::Secondary)
}

fn with_preview(request: &MeetRequest, head: &[String], buttons: Vec<Component>) -> MessageView {
    let lines: Vec<&str> = head.iter().map(String::as_str).filter(|line| !line.is_empty()).collect();
    let top = lines.join("\n");
    let room = MESSAGE_MAX.saturating_sub(utf16_length(&top) + 2);
    let preview = meet_post_content(&request.document, &[request.host_id], request.updated_at, room);
    MessageView {
        content: format!("{top}\n\n{preview}"),
        components: if buttons.is_empty() { Vec::new() } else { vec![row(buttons)] },
    }
}

fn reason_of(request: &MeetRequest) -> String {
    request
        .decline_reason
        .as_ref()
        .filter(|reason| !reason.is_empty())
        .map_or_else(String::new, |reason| format!(": {reason}"))
}

pub fn meet_request_card(request: &MeetRequest, outcome: MeetRequestOutcome) -> MessageView {
    let mut head = vec![
        format!("<@{}> wants to post this in <#{}>", request.host_id, request.forum_id),
        if request.host_edited { "-# Edited by the host".to_owned() } else { String::new() },
        if outcome == MeetRequestOutcome::Open && request.host_dm.is_none() {
            "-# Their DMs are closed, so I cannot tell them what happens.".to_owned()
        } else {
            String::new()
        },
    ];
    let status = match outcome {
        MeetRequestOutcome::Posted { by, thread_id } => format!("✅ Posted by <@{by}>: <#{thread_id}>"),
        MeetRequestOutcome::Withdrawn => "↩️ Withdrawn by the host".to_owned(),
        MeetRequestOutcome::Expired => EXPIRED.to_owned(),
        MeetRequestOutcome::Open if request.state == RequestState::Declined => {
            let by = request.handled_by.map_or_else(String::new, |id| id.to_string());
            format!("❌ Declined by <@{by}>{}", reason_of(request))
        }
        MeetRequestOutcome::Open => {
            let buttons = vec![
                button(meet_request_custom_id(MeetRequestAction::Post, &request.id), "Post", ButtonStyle::Primary),
                edit_button(request, "Edit"),
                button(meet_request_custom_id(MeetRequestAction::Decline, &request.id), "Decline", ButtonStyle::Danger),
            ];
            return with_preview(request, &head, buttons);
        }
    };
    head.push(status);
    with_preview(request, &head, Vec::new())
}

pub fn meet_request_host_view(request: &MeetRequest, outcome: MeetRequestOutcome, guild_name: &str) -> MessageView {
    let moderators = format!("the moderators of <#{}> in **{guild_name}**", request.forum_id);
    let withdraw = || button(meet_request_custom_id(MeetRequestAction::Withdraw, &request.id), "Withdraw", ButtonStyle::Secondary);
    match outcome {
        MeetRequestOutcome::Posted { thread_id, .. } => {
            let changes = if request.moderators_edited { ", with changes by the moderators" } else { "" };
            with_preview(request, &[format!("✅ Posted: <#{thread_id}>{changes}")], Vec::new())
        }
        MeetRequestOutcome::Withdrawn => with_preview(request, &["↩️ You withdrew it.".to_owned()], Vec::new()),
        MeetRequestOutcome::Expired => with_preview(request, &[EXPIRED.to_owned()], Vec::new()),
        MeetRequestOutcome::Open if request.state == RequestState::Declined => with_preview(
            request,
            &[format!("❌ Declined by {moderators}{}", reason_of(request))],
            vec![edit_button(request, "Edit and send again"), withdraw()],
        ),
        MeetRequestOutcome::Open => {
            let closed_dms = if request.host_dm.is_some() {
                String::new()
            } else {
                "-# Your DMs are closed, so this is the only place you can follow it.".to_owned()
            };
            with_preview(
                request,
                &[format!("Waiting for {moderators}."), closed_dms],
                vec![edit_button(request, "Edit"), withdraw()],
            )
        }
    }
}
