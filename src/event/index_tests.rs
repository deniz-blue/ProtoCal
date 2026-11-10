use regex::Regex;
use serde_json::json;
use twilight_model::id::Id;

use super::index::{ListedEvent, etag_of, event_document_body, open_evnt_index_of};
use crate::records::EventPost;

fn event(thread_id: u64, name: &str, updated_at: i64) -> EventPost {
    EventPost {
        thread_id: Id::new(thread_id),
        guild_id: Id::new(1),
        forum_id: Id::new(2),
        role_id: Id::new(3),
        attend_reaction: "👍".to_owned(),
        organizers: vec![],
        document: json!({ "v": "0.2", "id": format!("https://p.test/events/{thread_id}.evnt.json"), "name": { "und": name } }),
        updated_at,
    }
}

#[test]
fn an_index_lists_the_newest_thread_first_with_each_documents_etag() {
    let older = event(998, "Bar meet", 1_800_000_000);
    let newer = event(1001, "Picnic", 1_800_000_060);
    let index = open_evnt_index_of(vec![
        ListedEvent { event: &older, href: "/e/998".to_owned(), name: Some("Bar meet".to_owned()) },
        ListedEvent { event: &newer, href: "/e/1001".to_owned(), name: Some("Picnic".to_owned()) },
    ]);
    assert_eq!(
        index.to_string(),
        json!({
            "version": 1,
            "items": [
                { "href": "/e/1001", "name": "Picnic", "updatedAt": "2027-01-15T08:01:00.000Z", "etag": etag_of(&event_document_body(&newer)) },
                { "href": "/e/998", "name": "Bar meet", "updatedAt": "2027-01-15T08:00:00.000Z", "etag": etag_of(&event_document_body(&older)) },
            ],
        })
        .to_string()
    );
}

#[test]
fn etags_change_with_the_body_and_are_weak() {
    assert!(Regex::new(r#"^W/"[A-Za-z0-9_-]{22}"$"#).unwrap().is_match(&etag_of("a")));
    assert_ne!(etag_of("a"), etag_of("b"));
}

#[test]
fn an_etag_is_the_one_the_typescript_bot_gave_the_same_body() {
    let body = r#"{"v":"0.2","id":"https://p.test/events/1.evnt.json","name":{"und":"Bar meet"}}"#;
    assert_eq!(etag_of(body), "W/\"y5T0lEBN0DDL0F7et7Gi11\"");
}
