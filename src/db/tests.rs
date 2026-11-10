use rusqlite::Connection;
use serde_json::json;
use twilight_model::id::Id;

use super::{Database, RequestEditor, RequestMatch, rename_tables_to_event_names};
use crate::records::{CalendarLink, EventForum, EventPost, EventRequest, MessageAt, RequestState};

const TABLES_BEFORE_THE_EVENT_RENAME: &str = "
    CREATE TABLE `meet_forums` (
        `forum_id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `announce_role_id` text,
        `review_channel_id` text
    );
    CREATE TABLE `meets` (
        `thread_id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `forum_id` text NOT NULL,
        `role_id` text NOT NULL,
        `attend_reaction` text NOT NULL,
        `organizers` text NOT NULL,
        `document` text NOT NULL,
        `updated_at` integer NOT NULL
    );
    CREATE TABLE `meet_requests` (
        `id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `forum_id` text NOT NULL,
        `host_id` text NOT NULL,
        `document` text NOT NULL,
        `state` text NOT NULL,
        `handled_by` text,
        `decline_reason` text,
        `host_edited` integer NOT NULL,
        `moderators_edited` integer NOT NULL,
        `card_channel_id` text NOT NULL,
        `card_message_id` text NOT NULL,
        `dm_channel_id` text,
        `dm_message_id` text,
        `updated_at` integer NOT NULL
    );
";

fn event() -> EventPost {
    EventPost {
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

fn request() -> EventRequest {
    EventRequest {
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
fn rows_from_before_the_event_rename_are_found_under_the_new_names() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(TABLES_BEFORE_THE_EVENT_RENAME).unwrap();
    connection
        .execute(
            "INSERT INTO meets VALUES ('10', '1', '2', '3', '👋', '[\"4\"]', ?1, 1800000000)",
            [event().document.to_string()],
        )
        .unwrap();
    connection.execute("INSERT INTO meet_forums VALUES ('2', '1', '9', '8')", []).unwrap();
    connection
        .execute(
            "INSERT INTO meet_requests VALUES ('5f0c6a52', '1', '2', '3', ?1, 'waiting', NULL, NULL, 0, 0, '4', '5', '6', '7', 1800000000)",
            [request().document.to_string()],
        )
        .unwrap();
    rename_tables_to_event_names(&connection).unwrap();
    rename_tables_to_event_names(&connection).unwrap();
    let db = Database::of_connection(connection).unwrap();
    assert_eq!(db.event_of_thread(Id::new(10)).unwrap(), Some(event()));
    assert_eq!(
        db.event_forum_of(Id::new(2)).unwrap(),
        Some(EventForum {
            forum_id: Id::new(2),
            guild_id: Id::new(1),
            announce_role_id: Some(Id::new(9)),
            review_channel_id: Some(Id::new(8)),
        })
    );
    assert_eq!(db.event_request_of("5f0c6a52").unwrap(), Some(request()));
}

#[test]
fn an_event_is_stored_as_the_typescript_bot_stored_it() {
    let db = Database::in_memory();
    db.save_event(&event()).unwrap();
    assert_eq!(db.event_of_thread(Id::new(10)).unwrap(), Some(event()));
    let organizers: String = db
        .with(|db| db.query_row("SELECT organizers FROM events", [], |row| row.get(0)))
        .unwrap();
    assert_eq!(organizers, r#"["4"]"#);
    db.replace_event_role(Id::new(10), Id::new(30)).unwrap();
    assert_eq!(db.events_of_forum(Id::new(2)).unwrap()[0].role_id, Id::new(30));
    db.forget_event(Id::new(10)).unwrap();
    assert!(db.all_events().unwrap().is_empty());
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
    let forum = db.event_forum_of(Id::new(2)).unwrap().unwrap();
    assert_eq!(forum.announce_role_id, Some(Id::new(9)));
    assert_eq!(forum.review_channel_id, None);
    db.save_event_forum(&EventForum { review_channel_id: Some(Id::new(8)), ..forum }).unwrap();
    assert!(db.event_forums_of_guild(Id::new(1)).unwrap()[0].is_screened());
    assert!(db.forget_event_forum(Id::new(2)).unwrap());
    assert!(!db.forget_event_forum(Id::new(2)).unwrap());
}

#[test]
fn a_request_is_declined_once_and_a_host_edit_sends_it_again() {
    let db = Database::in_memory();
    db.save_event_request(&request()).unwrap();
    let declined = db.decline_event_request("5f0c6a52", Id::new(8), Some("no place")).unwrap().unwrap();
    assert_eq!(declined.state, RequestState::Declined);
    assert_eq!(declined.decline_reason.as_deref(), Some("no place"));
    assert_eq!(db.decline_event_request("5f0c6a52", Id::new(9), None).unwrap(), None);
    assert_eq!(db.forget_event_request("5f0c6a52", RequestMatch::OnlyIfWaiting).unwrap(), None);
    let document = json!({ "v": "0.2", "id": "urn:uuid:5f0c6a52", "name": { "und": "Picnic" } });
    let sent_again = db.edit_event_request("5f0c6a52", &document, RequestEditor::Host, 1).unwrap().unwrap();
    assert_eq!((sent_again.state, sent_again.handled_by, sent_again.host_edited), (RequestState::Waiting, None, true));
    let claimed = db.forget_event_request("5f0c6a52", RequestMatch::OnlyIfWaiting).unwrap().unwrap();
    assert_eq!(claimed.document, document);
    assert!(db.all_event_requests().unwrap().is_empty());
}

#[test]
fn a_reposted_card_moves_the_request_to_the_new_message() {
    let db = Database::in_memory();
    db.save_event_request(&request()).unwrap();
    let moved = db.move_event_request_card("5f0c6a52", Id::new(99)).unwrap().unwrap();
    assert_eq!(moved.card_message_id, Some(Id::new(99)));
    assert_eq!(db.event_request_of("5f0c6a52").unwrap().unwrap().card_message_id, Some(Id::new(99)));
    assert_eq!(db.move_event_request_card("gone", Id::new(99)).unwrap(), None);
}

#[test]
fn a_hosts_requests_are_those_in_the_guild_newest_first() {
    let db = Database::in_memory();
    let older = EventRequest { id: "older".to_owned(), updated_at: 1, ..request() };
    let newer = EventRequest { id: "newer".to_owned(), updated_at: 2, ..request() };
    let other_host = EventRequest { id: "other-host".to_owned(), host_id: Id::new(77), ..request() };
    let other_guild = EventRequest { id: "other-guild".to_owned(), guild_id: Id::new(78), ..request() };
    for request in [&older, &newer, &other_host, &other_guild] {
        db.save_event_request(request).unwrap();
    }
    let ids: Vec<String> = db.event_requests_of_host(request().guild_id, request().host_id).unwrap().into_iter().map(|request| request.id).collect();
    assert_eq!(ids, ["newer", "older"]);
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
