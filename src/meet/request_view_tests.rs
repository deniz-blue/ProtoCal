use regex::Regex;
use serde_json::json;
use twilight_model::id::Id;

use super::request_view::{
    MeetRequestAction, MeetRequestOutcome, MessageView, meet_request_action_of, meet_request_card,
    meet_request_custom_id, meet_request_host_view,
};
use crate::{
    components::button_labels,
    records::{MeetRequest, MessageAt, RequestState},
    text::utf16_length,
};

fn request() -> MeetRequest {
    MeetRequest {
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
    Regex::new(pattern).unwrap().is_match(&view.content)
}

#[test]
fn a_waiting_requests_card_asks_the_moderators_and_the_host_can_edit_or_withdraw() {
    let card = meet_request_card(&request(), MeetRequestOutcome::Open);
    assert!(matches(&card, r"^<@3> wants to post this in <#2>\n\n## Bar meet"));
    assert_eq!(button_labels(&card.components), ["Post", "Edit", "Decline"]);
    let host_view = meet_request_host_view(&request(), MeetRequestOutcome::Open, "LTFur");
    assert_eq!(button_labels(&host_view.components), ["Edit", "Withdraw"]);
}

#[test]
fn a_declined_request_shows_who_and_why_and_the_host_can_send_it_again() {
    let declined = MeetRequest {
        state: RequestState::Declined,
        handled_by: Some(Id::new(8)),
        decline_reason: Some("no place yet".to_owned()),
        ..request()
    };
    let card = meet_request_card(&declined, MeetRequestOutcome::Open);
    assert!(matches(&card, r"❌ Declined by <@8>: no place yet"));
    assert!(button_labels(&card.components).is_empty());
    let host_view = meet_request_host_view(&declined, MeetRequestOutcome::Open, "LTFur");
    assert!(matches(&host_view, r"^❌ Declined by the moderators of <#2> in \*\*LTFur\*\*: no place yet"));
    assert_eq!(button_labels(&host_view.components), ["Edit and send again", "Withdraw"]);
}

#[test]
fn a_posted_request_links_the_post_on_both_sides_and_has_no_buttons() {
    let posted = MeetRequestOutcome::Posted { by: Id::new(8), thread_id: Id::new(9) };
    let edited = MeetRequest { moderators_edited: true, ..request() };
    let card = meet_request_card(&edited, posted);
    assert!(matches(&card, r"✅ Posted by <@8>: <#9>"));
    assert!(card.components.is_empty());
    let host_view = meet_request_host_view(&edited, posted, "LTFur");
    assert!(matches(&host_view, r"^✅ Posted: <#9>, with changes by the moderators"));
}

#[test]
fn closed_dms_are_noted_on_the_card_and_in_the_hosts_reply() {
    let closed = MeetRequest { host_dm: None, ..request() };
    assert!(matches(&meet_request_card(&closed, MeetRequestOutcome::Open), "DMs are closed"));
    assert!(matches(&meet_request_host_view(&closed, MeetRequestOutcome::Open, "LTFur"), "DMs are closed"));
}

#[test]
fn a_long_description_still_fits_one_message_under_the_cards_head() {
    let long = MeetRequest {
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
    assert!(utf16_length(&meet_request_card(&long, MeetRequestOutcome::Open).content) <= 2000);
    assert!(utf16_length(&meet_request_host_view(&long, MeetRequestOutcome::Open, "LTFur").content) <= 2000);
}

#[test]
fn button_ids_carry_the_action_and_the_request() {
    let id = request().id;
    assert_eq!(
        meet_request_action_of(&meet_request_custom_id(MeetRequestAction::Withdraw, &id)),
        Some((MeetRequestAction::Withdraw, id.clone()))
    );
    assert_eq!(meet_request_action_of("meet-form:request:x"), None);
    assert!(meet_request_custom_id(MeetRequestAction::DeclineReason, &id).len() <= 100);
}
