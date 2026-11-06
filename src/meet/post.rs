use serde_json::Value;
use twilight_model::{
    channel::message::{Component, component::ButtonStyle},
    id::{
        Id,
        marker::{ChannelMarker, UserMarker},
    },
};

use super::{
    change_notice::meet_status_as_discord,
    dates::meet_dates_as_discord,
    document::{meet_description_of, meet_name_of, meet_place_of},
};
use crate::{
    components::{button, link_button, row},
    config::Links,
    text::{cut_to_utf16_length, utf16_length},
};

// Discord limits: a message is 2000 characters, a thread name 100.
pub const MESSAGE_MAX: usize = 2000;
pub const THREAD_NAME_MAX: usize = 100;

pub const MEET_FORM_BUTTON: &str = "meet-form";
pub const MEET_EDIT_BUTTON: &str = "meet-edit";
pub const MEET_FORUM_PICK: &str = "meet-forum-pick";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeetFormTarget {
    New(Id<ChannelMarker>),
    Edit(Id<ChannelMarker>),
    Request(String),
}

impl MeetFormTarget {
    pub fn key(&self) -> String {
        match self {
            Self::New(forum_id) => format!("new:{forum_id}"),
            Self::Edit(thread_id) => format!("edit:{thread_id}"),
            Self::Request(request_id) => format!("request:{request_id}"),
        }
    }

    pub fn of_key(key: &str) -> Option<Self> {
        let (kind, id) = key.split_once(':')?;
        match kind {
            "new" => id.parse().ok().map(Self::New),
            "edit" => id.parse().ok().map(Self::Edit),
            "request" if !id.is_empty() => Some(Self::Request(id.to_owned())),
            _ => None,
        }
    }

    pub fn custom_id(&self) -> String {
        format!("{MEET_FORM_BUTTON}:{}", self.key())
    }

    pub fn of_custom_id(custom_id: &str) -> Option<Self> {
        Self::of_key(custom_id.strip_prefix(MEET_FORM_BUTTON)?.strip_prefix(':')?)
    }
}

pub fn thread_of_edit_button(custom_id: &str) -> Option<Id<ChannelMarker>> {
    custom_id
        .strip_prefix(MEET_EDIT_BUTTON)?
        .strip_prefix(':')?
        .parse()
        .ok()
}

pub fn thread_name_of(document: &Value) -> String {
    cut_to_utf16_length(&meet_name_of(document), THREAD_NAME_MAX).to_owned()
}

pub fn meet_post_content(
    document: &Value,
    organizers: &[Id<UserMarker>],
    updated_at_unix_seconds: i64,
    max_length: usize,
) -> String {
    let mentions: Vec<String> = organizers.iter().map(|id| format!("<@{id}>")).collect();
    let facts: Vec<String> = [
        meet_status_as_discord(document).map(str::to_owned),
        meet_dates_as_discord(document, true).map(|dates| format!("📆 **Date & Time:** {dates}")),
        meet_place_of(document)
            .filter(|place| !place.is_empty())
            .map(|place| format!("📍 **Location:** {place}")),
        Some(format!("🪧 **Organized by** {}", mentions.join(" "))),
    ]
    .into_iter()
    .flatten()
    .collect();
    let head = format!("## {}\n\n{}", meet_name_of(document), facts.join("\n"));
    let foot = format!("-# Last updated at <t:{updated_at_unix_seconds}:R>");
    let Some(description) = meet_description_of(document).filter(|text| !text.is_empty()) else {
        return format!("{head}\n\n{foot}");
    };
    let room = max_length.saturating_sub(utf16_length(&head) + utf16_length(&foot) + 4);
    let shown = if utf16_length(&description) > room {
        format!("{}…", cut_to_utf16_length(&description, room.saturating_sub(1)))
    } else {
        description
    };
    format!("{head}\n\n{shown}\n\n{foot}")
}

pub fn meet_post_buttons(links: &Links, thread_id: Id<ChannelMarker>, viewable: bool) -> Component {
    let edit = button(format!("{MEET_EDIT_BUTTON}:{thread_id}"), "Edit", ButtonStyle::Secondary);
    let view = link_button("View", links.meet_view_url(thread_id));
    row(if viewable { vec![view, edit] } else { vec![edit] })
}

pub fn meet_edit_buttons(thread_id: Id<ChannelMarker>, edit_url: String) -> Component {
    row(vec![
        button(
            MeetFormTarget::Edit(thread_id).custom_id(),
            "Edit on Discord",
            ButtonStyle::Secondary,
        ),
        link_button("Edit via App", edit_url),
    ])
}
