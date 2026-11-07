use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use jiff::Timestamp;
use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};

use crate::records::EventPost;

pub fn etag_of(body: &str) -> String {
    let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(body.as_bytes()));
    format!("W/\"{}\"", &digest[..22])
}

pub fn event_document_body(event: &EventPost) -> String {
    event.document.to_string()
}

fn iso_of_unix_seconds(seconds: i64) -> String {
    Timestamp::from_second(seconds)
        .unwrap_or(Timestamp::UNIX_EPOCH)
        .strftime("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

pub struct ListedEvent<'a> {
    pub event: &'a EventPost,
    pub href: String,
    pub name: Option<String>,
}

pub fn open_evnt_index_of(mut listed: Vec<ListedEvent>) -> Value {
    listed.sort_by_key(|listed| std::cmp::Reverse(listed.event.thread_id.get()));
    let items: Vec<Value> = listed
        .into_iter()
        .map(|ListedEvent { event, href, name }| {
            let mut item = Map::new();
            item.insert("href".to_owned(), json!(href));
            if let Some(name) = name {
                item.insert("name".to_owned(), json!(name));
            }
            item.insert("updatedAt".to_owned(), json!(iso_of_unix_seconds(event.updated_at)));
            item.insert("etag".to_owned(), json!(etag_of(&event_document_body(event))));
            Value::Object(item)
        })
        .collect();
    json!({ "version": 1, "items": items })
}
