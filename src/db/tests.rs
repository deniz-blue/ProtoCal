use rusqlite::Connection;
use serde_json::json;
use twilight_model::id::Id;

use super::{Database, RequestEditor, RequestMatch};
use crate::records::{CalendarLink, Meet, MeetForum, MeetRequest, MessageAt, RequestState};

fn meet() -> Meet {
    Meet {
        thread_id: Id::new(10),
        guild_id: Id::new(1),
        forum_id: Id::new(2),
        role_id: Id::new(3),
        attend_reaction: "👋".to_owned(),
        organizers: vec![Id::new(4)],
        document: json!({ "v": "0.2", "id": "https://p.test/events/10.evnt.json", "name": { "und": "Bar meet" } }),
        updated_at: 1_800_000_000,
    }
}

fn request() -> MeetRequest {
    MeetRequest {
        id: "5f0c6a52".to_owned(),
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

#[test]
fn a_meet_is_stored_as_the_typescript_bot_stored_it() {
    let db = Database::in_memory();
    db.save_meet(&meet()).unwrap();
    assert_eq!(db.meet_of_thread(Id::new(10)).unwrap(), Some(meet()));
    let organizers: String = db
        .with(|db| db.query_row("SELECT organizers FROM meets", [], |row| row.get(0)))
        .unwrap();
    assert_eq!(organizers, r#"["4"]"#);
    db.replace_meet_role(Id::new(10), Id::new(30)).unwrap();
    assert_eq!(db.meets_of_forum(Id::new(2)).unwrap()[0].role_id, Id::new(30));
    db.forget_meet(Id::new(10)).unwrap();
    assert!(db.all_meets().unwrap().is_empty());
}

#[test]
fn a_database_from_before_screening_gets_the_review_channel_column() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE `meet_forums` (`forum_id` text PRIMARY KEY NOT NULL, `guild_id` text NOT NULL, `announce_role_id` text);
             INSERT INTO meet_forums VALUES ('2', '1', '9');",
        )
        .unwrap();
    let db = Database::of_connection(connection).unwrap();
    let forum = db.meet_forum_of(Id::new(2)).unwrap().unwrap();
    assert_eq!(forum.announce_role_id, Some(Id::new(9)));
    assert_eq!(forum.review_channel_id, None);
    db.save_meet_forum(&MeetForum { review_channel_id: Some(Id::new(8)), ..forum }).unwrap();
    assert!(db.meet_forums_of_guild(Id::new(1)).unwrap()[0].is_screened());
    assert!(db.forget_meet_forum(Id::new(2)).unwrap());
    assert!(!db.forget_meet_forum(Id::new(2)).unwrap());
}

#[test]
fn a_request_is_declined_once_and_a_host_edit_sends_it_again() {
    let db = Database::in_memory();
    db.save_meet_request(&request()).unwrap();
    let declined = db.decline_meet_request("5f0c6a52", Id::new(8), Some("no place")).unwrap().unwrap();
    assert_eq!(declined.state, RequestState::Declined);
    assert_eq!(declined.decline_reason.as_deref(), Some("no place"));
    assert_eq!(db.decline_meet_request("5f0c6a52", Id::new(9), None).unwrap(), None);
    assert_eq!(db.forget_meet_request("5f0c6a52", RequestMatch::OnlyIfWaiting).unwrap(), None);
    let document = json!({ "v": "0.2", "id": "urn:uuid:5f0c6a52", "name": { "und": "Picnic" } });
    let sent_again = db.edit_meet_request("5f0c6a52", &document, RequestEditor::Host, 1).unwrap().unwrap();
    assert_eq!((sent_again.state, sent_again.handled_by, sent_again.host_edited), (RequestState::Waiting, None, true));
    let claimed = db.forget_meet_request("5f0c6a52", RequestMatch::OnlyIfWaiting).unwrap().unwrap();
    assert_eq!(claimed.document, document);
    assert!(db.all_meet_requests().unwrap().is_empty());
}

#[test]
fn a_calendar_link_is_found_by_person_and_by_token_and_replaced_in_place() {
    let db = Database::in_memory();
    let link = CalendarLink { user_id: Id::new(4), token: "abc".to_owned(), created_at: 1 };
    db.save_calendar_link(&link).unwrap();
    db.save_calendar_link(&CalendarLink { token: "def".to_owned(), ..link.clone() }).unwrap();
    assert_eq!(db.calendar_link_of_token("abc").unwrap(), None);
    assert_eq!(db.calendar_link_of_user(Id::new(4)).unwrap().unwrap().token, "def");
}
