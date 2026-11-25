use serde_json::Value;
use twilight_model::{
    channel::message::{
        Component, EmojiReactionType,
        component::{Button, ButtonStyle},
    },
    id::{
        Id,
        marker::{ChannelMarker, UserMarker},
    },
};

use super::{
    attend_reaction::emoji_text_of_reaction_key,
    change_notice::event_status_as_discord,
    dates::event_dates_as_discord,
    document::{event_coordinates_of, event_description_of, event_name_of, event_place_of},
    parent::PartOf,
};
use crate::{
    components::{button, link_button, row},
    config::{Links, with_query},
    text::{cut_to_utf16_length, utf16_length},
};

// Discord limits: a message is 2000 characters, a thread name 100.
pub const MESSAGE_MAX: usize = 2000;
pub const THREAD_NAME_MAX: usize = 100;
const MAP_SEARCH_URL: &str = "https://www.google.com/maps/search/?api=1";

// Buttons already posted to Discord carry these ids.
pub const EVENT_FORM_BUTTON: &str = "meet-form";
pub const EVENT_EDIT_BUTTON: &str = "meet-edit";
pub const EVENT_FORUM_PICK: &str = "meet-forum-pick";
pub const NEW_EVENT_BUTTON: &str = "event-new";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewEventButton {
    pub forum_id: Option<Id<ChannelMarker>>,
}

impl NewEventButton {
    pub fn custom_id(self) -> String {
        match self.forum_id {
            Some(forum_id) => format!("{NEW_EVENT_BUTTON}:{forum_id}"),
            None => NEW_EVENT_BUTTON.to_owned(),
        }
    }

    pub fn of_custom_id(custom_id: &str) -> Option<Self> {
        match custom_id.strip_prefix(NEW_EVENT_BUTTON)? {
            "" => Some(Self { forum_id: None }),
            rest => rest.strip_prefix(':')?.parse().ok().map(|forum_id| Self { forum_id: Some(forum_id) }),
        }
    }

    pub fn component(self, label: &str, emoji: Option<EmojiReactionType>, style: ButtonStyle) -> Component {
        row(vec![Component::Button(Button {
            id: None,
            custom_id: Some(self.custom_id()),
            disabled: false,
            emoji,
            label: Some(label.to_owned()),
            style,
            url: None,
            sku_id: None,
        })])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventFormTarget {
    New(Id<ChannelMarker>),
    Edit(Id<ChannelMarker>),
    Request(String),
}

impl EventFormTarget {
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
        format!("{EVENT_FORM_BUTTON}:{}", self.key())
    }

    pub fn of_custom_id(custom_id: &str) -> Option<Self> {
        Self::of_key(custom_id.strip_prefix(EVENT_FORM_BUTTON)?.strip_prefix(':')?)
    }
}

pub fn thread_of_edit_button(custom_id: &str) -> Option<Id<ChannelMarker>> {
    custom_id
        .strip_prefix(EVENT_EDIT_BUTTON)?
        .strip_prefix(':')?
        .parse()
        .ok()
}

pub fn thread_name_of(document: &Value) -> String {
    cut_to_utf16_length(&event_name_of(document), THREAD_NAME_MAX).to_owned()
}

fn place_as_discord(document: &Value) -> Option<String> {
    let place = event_place_of(document).filter(|place| !place.is_empty())?;
    let query = event_coordinates_of(document).map_or_else(|| place.clone(), |(latitude, longitude)| format!("{latitude},{longitude}"));
    let map_url = with_query(MAP_SEARCH_URL, &[("query", &query)]);
    Some(format!("[{}](<{map_url}>)", place.replace(['[', ']'], "")))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventPostText {
    pub head: String,
    pub description: Option<String>,
    pub foot: String,
}

impl EventPostText {
    pub fn description_within(&self, longest: usize) -> Option<String> {
        let description = self.description.as_ref()?;
        if utf16_length(description) <= longest {
            return Some(description.clone());
        }
        Some(format!("{}…", cut_to_utf16_length(description, longest.saturating_sub(1))))
    }

    pub fn as_content(&self, max_length: usize) -> String {
        let room = max_length.saturating_sub(utf16_length(&self.head) + utf16_length(&self.foot) + 4);
        match self.description_within(room) {
            Some(description) => format!("{}\n\n{description}\n\n{}", self.head, self.foot),
            None => format!("{}\n\n{}", self.head, self.foot),
        }
    }
}

pub fn event_post_text(
    document: &Value,
    part_of: Option<&PartOf>,
    organizers: &[Id<UserMarker>],
    attend_reaction: Option<&str>,
    updated_at_unix_seconds: i64,
) -> EventPostText {
    let mentions: Vec<String> = organizers.iter().map(|id| format!("<@{id}>")).collect();
    let facts: Vec<String> = [
        event_status_as_discord(document).map(str::to_owned),
        part_of.map(PartOf::line),
        Some(format!("🪧 **Organized by** {}", mentions.join(" "))),
        event_dates_as_discord(document, true).map(|dates| format!("📆 **Date & Time:** {dates}")),
        place_as_discord(document).map(|place| format!("📍 **Location:** {place}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let updated = format!("-# Last updated at <t:{updated_at_unix_seconds}:R>");
    let foot = match attend_reaction {
        Some(key) => format!("{updated}\n-# React {} to get the role", emoji_text_of_reaction_key(key)),
        None => updated,
    };
    EventPostText {
        head: format!("## {}\n\n{}", event_name_of(document), facts.join("\n")),
        description: event_description_of(document).filter(|text| !text.is_empty()),
        foot,
    }
}

pub fn event_post_content(
    document: &Value,
    part_of: Option<&PartOf>,
    organizers: &[Id<UserMarker>],
    attend_reaction: Option<&str>,
    updated_at_unix_seconds: i64,
    max_length: usize,
) -> String {
    event_post_text(document, part_of, organizers, attend_reaction, updated_at_unix_seconds).as_content(max_length)
}

pub fn event_post_buttons(links: &Links, thread_id: Id<ChannelMarker>, link_id: &str) -> Component {
    let edit = button(format!("{EVENT_EDIT_BUTTON}:{thread_id}"), "Edit", ButtonStyle::Secondary);
    row(vec![link_button("View", links.event_view_url(link_id)), edit])
}

pub fn has_link_button(components: &[Component], url: &str) -> bool {
    components.iter().any(|component| match component {
        Component::ActionRow(row) => has_link_button(&row.components, url),
        Component::Button(Button { url: Some(button_url), .. }) => button_url == url,
        _ => false,
    })
}

pub fn event_edit_buttons(thread_id: Id<ChannelMarker>, edit_url: String) -> Component {
    row(vec![
        button(
            EventFormTarget::Edit(thread_id).custom_id(),
            "Edit on Discord",
            ButtonStyle::Secondary,
        ),
        link_button("Edit via App", edit_url),
    ])
}

#[cfg(test)]
mod tests {
    use twilight_model::{channel::message::EmojiReactionType, id::Id};

    use serde_json::json;

    use super::{EventFormTarget, NewEventButton, event_post_buttons, event_post_content, has_link_button};
    use crate::{components::emoji_of_text, config::test_links, open_evnt::COORDINATES_TYPE};

    #[test]
    fn the_location_links_to_a_map_search_for_its_coordinates_or_else_its_text() {
        let by_text = json!({ "venue": { "address": "Vilniaus g. 22, Kaunas [2nd floor]" } });
        assert!(event_post_content(&by_text, None, &[], None, 0, 2000).contains(
            "📍 **Location:** [Vilniaus g. 22, Kaunas 2nd floor](<https://www.google.com/maps/search/?api=1&query=Vilniaus+g.+22%2C+Kaunas+%5B2nd+floor%5D>)"
        ));
        let coordinates = json!({ "$type": COORDINATES_TYPE, "latitude": 54.8972, "longitude": 23.8861 });
        let by_coordinates = json!({ "venue": { "name": { "und": "Bar" }, "components": [coordinates] } });
        assert!(event_post_content(&by_coordinates, None, &[], None, 0, 2000).contains(
            "📍 **Location:** [Bar](<https://www.google.com/maps/search/?api=1&query=54.8972%2C23.8861>)"
        ));
    }

    #[test]
    fn a_post_has_the_link_button_of_its_own_link_id_only() {
        let links = test_links();
        let buttons = [event_post_buttons(&links, Id::new(10), "new")];
        assert!(has_link_button(&buttons, &links.event_view_url("new")));
        assert!(!has_link_button(&buttons, &links.event_view_url("old")));
    }

    #[test]
    fn emoji_of_text_reads_custom_and_standard_emoji() {
        let party = EmojiReactionType::Custom { animated: false, id: Id::new(123), name: Some("party".to_owned()) };
        assert_eq!(emoji_of_text("<:party:123>"), party);
        let spin = EmojiReactionType::Custom { animated: true, id: Id::new(9), name: Some("spin".to_owned()) };
        assert_eq!(emoji_of_text(" <a:spin:9> "), spin);
        assert_eq!(emoji_of_text("📅"), EmojiReactionType::Unicode { name: "📅".to_owned() });
        assert_eq!(emoji_of_text("<:party:x>"), EmojiReactionType::Unicode { name: "<:party:x>".to_owned() });
    }

    #[test]
    fn new_event_button_reads_back_its_custom_id() {
        for button in [NewEventButton { forum_id: None }, NewEventButton { forum_id: Some(Id::new(42)) }] {
            assert_eq!(NewEventButton::of_custom_id(&button.custom_id()), Some(button));
        }
        assert_eq!(NewEventButton::of_custom_id("event-new:x"), None);
        assert_eq!(NewEventButton::of_custom_id("event-newer"), None);
        assert_eq!(NewEventButton::of_custom_id(&EventFormTarget::New(Id::new(42)).custom_id()), None);
    }
}
