use regex::Regex;
use serde_json::json;
use twilight_model::{
    channel::message::{Component, component::ButtonStyle},
    id::Id,
};

use super::request_view::{
    EventRequestAction, EventRequestOutcome, MessageView, TEXT_DISPLAYS_MAX, changes_asked_message,
    event_request_action_of, event_request_card, event_request_custom_id, event_request_host_view, host_requests_view, is_host_requests_list,
};
use super::post::MESSAGE_MAX;
use crate::{
    components::{COMPONENTS_MAX, button, button_labels, component_count, row, text_displays},
    records::{EventRequest, MessageAt, RequestState},
    text::utf16_length,
};

fn request() -> EventRequest {
    EventRequest {
        id: "5f0c6a52-6d0e-4a8e-9a3b-6f3e2c1d0b9a".to_owned(),
        guild_id: Id::new(1),
        forum_id: Id::new(2),
        host_id: Id::new(3),
        document: json!({ "v": "0.2", "id": "urn:uuid:5f0c6a52", "name": { "und": "Bar meet" } }),
        state: RequestState::Waiting,
        handled_by: None,
        decline_reason: None,
        host_edited: false,
        moderators_edited: false,
        card_channel_id: Id::new(4),
        card_message_id: Some(Id::new(5)),
        host_dm: Some(MessageAt { channel_id: Id::new(6), message_id: Id::new(7) }),
        updated_at: 1_800_000_000,
    }
}

fn matches(view: &MessageView, pattern: &str) -> bool {
    Regex::new(pattern).unwrap().is_match(&text_displays(&view.components).join("\n\n"))
}

fn event_and_head(view: &MessageView) -> (String, String) {
    let [Component::Container(event), Component::TextDisplay(head), ..] = view.components.as_slice() else {
        panic!("expected a container and then a text display: {:?}", view.components);
    };
    (text_displays(&event.components).join("\n\n"), head.content.clone())
}

#[test]
fn a_waiting_requests_card_asks_the_moderators_and_the_host_can_edit_or_withdraw() {
    let card = event_request_card(&request(), EventRequestOutcome::Open);
    let (event, head) = event_and_head(&card);
    assert_eq!(head, "<@3> wants to post this in <#2>");
    assert!(event.starts_with("## Bar meet"));
    assert_eq!(button_labels(&card.components), ["Approve", "Edit", "Reply", "Deny"]);
    let host_view = event_request_host_view(&request(), EventRequestOutcome::Open, "LTFur");
    assert_eq!(button_labels(&host_view.components), ["Edit", "Withdraw"]);
}

#[test]
fn a_declined_request_shows_who_and_why_and_the_host_can_send_it_again() {
    let declined = EventRequest {
        state: RequestState::Declined,
        handled_by: Some(Id::new(8)),
        decline_reason: Some("no place yet".to_owned()),
        ..request()
    };
    let card = event_request_card(&declined, EventRequestOutcome::Open);
    assert!(matches(&card, r"❌ Denied by <@8>: no place yet"));
    assert!(button_labels(&card.components).is_empty());
    let host_view = event_request_host_view(&declined, EventRequestOutcome::Open, "LTFur");
    assert!(matches(&host_view, r"(?m)^❌ Denied by the moderators of <#2> in \*\*LTFur\*\*: no place yet"));
    assert_eq!(button_labels(&host_view.components), ["Edit and send again", "Withdraw"]);
}

#[test]
fn a_posted_request_links_the_post_on_both_sides_and_has_no_buttons() {
    let posted = EventRequestOutcome::Posted { by: Id::new(8), thread_id: Id::new(9) };
    let edited = EventRequest { moderators_edited: true, ..request() };
    let card = event_request_card(&edited, posted);
    assert!(matches(&card, r"✅ Approved by <@8>: <#9>"));
    assert!(button_labels(&card.components).is_empty());
    let host_view = event_request_host_view(&edited, posted, "LTFur");
    assert!(matches(&host_view, r"(?m)^✅ Posted: <#9>, with changes by the moderators"));
}

#[test]
fn closed_dms_are_noted_on_the_card_and_in_the_hosts_reply() {
    let closed = EventRequest { host_dm: None, ..request() };
    assert!(matches(&event_request_card(&closed, EventRequestOutcome::Open), "DMs are closed"));
    assert!(matches(&event_request_host_view(&closed, EventRequestOutcome::Open, "LTFur"), "DMs are closed"));
}

#[test]
fn a_long_description_fits_the_card_and_the_event_part_fits_a_post() {
    let long = EventRequest {
        host_edited: true,
        moderators_edited: true,
        document: json!({
            "v": "0.2",
            "id": "urn:uuid:5f0c6a52",
            "name": { "und": "Bar meet" },
            "components": [{ "$type": "directory.evnt.text.markdown#0", "content": { "und": "x".repeat(3000) } }],
        }),
        ..request()
    };
    for view in [event_request_card(&long, EventRequestOutcome::Open), event_request_host_view(&long, EventRequestOutcome::Open, "LTFur")] {
        let texts = text_displays(&view.components);
        assert!(texts.iter().map(|text| utf16_length(text)).sum::<usize>() <= TEXT_DISPLAYS_MAX);
        assert!(utf16_length(&event_and_head(&view).0) <= MESSAGE_MAX);
    }
}

#[test]
fn the_hosts_requests_list_each_event_then_its_state_and_end_with_a_new_event_button() {
    let dated = EventRequest {
        document: json!({
            "v": "0.2",
            "id": "urn:uuid:5f0c6a52",
            "name": { "und": "Bar meet" },
            "occurrences": [{ "start": "2026-12-05T18:00[Europe/Vilnius]" }],
        }),
        host_dm: None,
        ..request()
    };
    let declined = EventRequest {
        id: "other".to_owned(),
        state: RequestState::Declined,
        decline_reason: Some("no place yet".to_owned()),
        document: json!({ "v": "0.2", "id": "urn:uuid:other", "name": { "und": "Picnic" } }),
        host_dm: None,
        ..request()
    };
    let new_event = row(vec![button("meet-form:new:2", "New event", ButtonStyle::Primary)]);
    let view = host_requests_view(&[dated, declined], "LTFur", vec![new_event]);
    assert_eq!(
        text_displays(&view.components)[1..],
        [
            "**Bar meet**\n2026-12-05 18:00".to_owned(),
            "Waiting for the moderators of <#2> in **LTFur**.".to_owned(),
            "**Picnic**".to_owned(),
            "❌ Denied by the moderators of <#2> in **LTFur**: no place yet".to_owned(),
        ]
    );
    assert_eq!(button_labels(&view.components), ["Edit", "Withdraw", "Edit and send again", "Withdraw", "New event"]);
    assert!(matches!(view.components.get(1), Some(Component::Container(_))));
}

#[test]
fn the_hosts_requests_stop_before_discords_limits_and_keep_the_new_event_button() {
    let many: Vec<EventRequest> = (0..20).map(|n| EventRequest { id: n.to_string(), host_dm: None, ..request() }).collect();
    let new_event = row(vec![button("meet-form:new:2", "New event", ButtonStyle::Primary)]);
    let view = host_requests_view(&many, "LTFur", vec![new_event]);
    assert!(component_count(&view.components) <= COMPONENTS_MAX);
    assert_eq!(button_labels(&view.components).last().map(String::as_str), Some("New event"));
    assert!(button_labels(&view.components).len() > 2);
}

#[test]
fn the_hosts_requests_list_is_told_apart_from_a_single_request_and_can_be_empty() {
    let new_event = row(vec![button("meet-form:new:2", "New event", ButtonStyle::Primary)]);
    let empty = host_requests_view(&[], "LTFur", vec![new_event]);
    assert!(is_host_requests_list(&empty.components));
    assert_eq!(text_displays(&empty.components), ["Nothing of yours is waiting."]);
    assert_eq!(button_labels(&empty.components), ["New event"]);
    let closed = EventRequest { host_dm: None, ..request() };
    assert!(!is_host_requests_list(&event_request_host_view(&closed, EventRequestOutcome::Open, "LTFur").components));
}

#[test]
fn button_ids_carry_the_action_and_the_request() {
    let id = request().id;
    assert_eq!(
        event_request_action_of(&event_request_custom_id(EventRequestAction::Withdraw, &id)),
        Some((EventRequestAction::Withdraw, id.clone()))
    );
    assert_eq!(event_request_action_of("meet-form:request:x"), None);
    assert!(event_request_custom_id(EventRequestAction::DeclineReason, &id).len() <= 100);
    for action in [EventRequestAction::Reply, EventRequestAction::ReplyMessage] {
        assert_eq!(event_request_action_of(&event_request_custom_id(action, &id)), Some((action, id.clone())));
    }
}

#[test]
fn changes_asked_names_the_event_the_moderator_and_the_server() {
    assert_eq!(
        changes_asked_message("Bar meet", Id::new(8), "LTFur", "Which bar?"),
        "-# ✏️ Changes asked for **Bar meet** by <@8> at **LTFur**\nWhich bar?"
    );
}
