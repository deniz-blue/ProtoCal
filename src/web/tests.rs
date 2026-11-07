use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::Response,
};
use http_body_util::BodyExt as _;
use serde_json::{Value, json};
use tower::ServiceExt as _;
use twilight_http::Client;
use twilight_model::id::Id;

use super::{config_form::guild_config_change_of_form, router};
use crate::{
    bot::{Bot, EditGrant},
    config::test_links,
    db::Database,
    discord::fixtures::{GUILD, forum, guild, hidden_forum},
    records::{CalendarLink, EventForum, EventPost},
};

const OPEN_FORUM: u64 = 20;
const HIDDEN_FORUM: u64 = 21;

fn event(thread_id: u64, forum_id: u64, name: &str) -> EventPost {
    EventPost {
        thread_id: Id::new(thread_id),
        guild_id: Id::new(GUILD),
        forum_id: Id::new(forum_id),
        role_id: Id::new(70),
        attend_reaction: "👋".to_owned(),
        organizers: vec![Id::new(4)],
        document: json!({ "v": "0.2", "id": format!("https://p.test/events/{thread_id}.evnt.json"), "name": { "und": name } }),
        updated_at: 1_800_000_000,
    }
}

pub fn seeded_bot() -> Arc<Bot> {
    let unreachable = Client::builder().proxy("127.0.0.1:9".to_owned(), true).build();
    let db = Database::in_memory();
    for forum_id in [OPEN_FORUM, HIDDEN_FORUM] {
        db.save_event_forum(&EventForum {
            forum_id: Id::new(forum_id),
            guild_id: Id::new(GUILD),
            announce_role_id: None,
            review_channel_id: None,
        })
        .unwrap();
    }
    db.save_event(&event(1000, OPEN_FORUM, "Bar meet")).unwrap();
    db.save_event(&event(1001, OPEN_FORUM, "Park meet")).unwrap();
    db.save_event(&event(1002, HIDDEN_FORUM, "Secret meet")).unwrap();
    db.save_calendar_link(&CalendarLink { user_id: Id::new(4), token: "tok_en-1".to_owned(), created_at: 1 }).unwrap();
    let bot = Bot::new(unreachable, Id::new(1), Id::new(2), db, test_links());
    bot.cache.insert_guild(guild());
    bot.cache.insert_channel(forum(OPEN_FORUM, "meets", vec![]));
    bot.cache.insert_channel(hidden_forum(HIDDEN_FORUM, "members-meets"));
    Arc::new(bot)
}

pub async fn send(bot: &Arc<Bot>, request: Request<Body>) -> Response {
    router(Arc::clone(bot)).oneshot(request).await.unwrap()
}

async fn get(bot: &Arc<Bot>, path: &str) -> Response {
    send(bot, Request::get(path).body(Body::empty()).unwrap()).await
}

pub async fn body_of(response: Response) -> String {
    String::from_utf8(response.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap()
}

pub fn header_of(response: &Response, name: header::HeaderName) -> &str {
    response.headers().get(name).map_or("", |value| value.to_str().unwrap())
}

#[tokio::test]
async fn event_document_is_served_with_an_etag_and_cors() {
    let bot = seeded_bot();
    let response = get(&bot, "/events/1000.evnt.json").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header_of(&response, header::CONTENT_TYPE), "application/vnd.evnt+json");
    assert_eq!(header_of(&response, header::CACHE_CONTROL), "no-cache");
    assert_eq!(header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN), "*");
    let etag = header_of(&response, header::ETAG).to_owned();
    assert!(etag.starts_with("W/\""));
    let body: Value = serde_json::from_str(&body_of(response).await).unwrap();
    assert_eq!(body["name"]["und"], "Bar meet");

    let request = Request::get("/events/1000.evnt.json")
        .header(header::IF_NONE_MATCH, format!("\"other\", {etag}"))
        .body(Body::empty())
        .unwrap();
    let unchanged = send(&bot, request).await;
    assert_eq!(unchanged.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(header_of(&unchanged, header::ETAG), etag);
}

#[tokio::test]
async fn hidden_and_malformed_documents_are_not_found() {
    let bot = seeded_bot();
    for path in ["/events/1002.evnt.json", "/events/9.evnt.json", "/events/x1.evnt.json", "/forums/21.json", "/nowhere"] {
        let response = get(&bot, path).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(body_of(response).await, "404 Not Found");
    }
}

#[tokio::test]
async fn forum_index_lists_newest_first() {
    let bot = seeded_bot();
    let response = get(&bot, "/forums/20.json").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header_of(&response, header::CONTENT_TYPE), "application/json; charset=utf-8");
    let index: Value = serde_json::from_str(&body_of(response).await).unwrap();
    let hrefs: Vec<&str> = index["items"].as_array().unwrap().iter().map(|item| item["href"].as_str().unwrap()).collect();
    assert_eq!(hrefs, ["https://p.test/events/1001.evnt.json", "https://p.test/events/1000.evnt.json"]);
}

#[tokio::test]
async fn calendar_skips_guilds_whose_member_is_unknown() {
    let bot = seeded_bot();
    let response = get(&bot, "/calendars/tok_en-1.json").await;
    assert_eq!(response.status(), StatusCode::OK);
    let index: Value = serde_json::from_str(&body_of(response).await).unwrap();
    assert_eq!(index["items"], json!([]));
    assert_eq!(get(&bot, "/calendars/unknown.json").await.status(), StatusCode::NOT_FOUND);
    assert_eq!(get(&bot, "/calendars/tok_en-1/events/1000.evnt.json").await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn edit_link_redirects_to_the_editor_while_granted() {
    let bot = seeded_bot();
    let expired = get(&bot, "/edit/nope").await;
    assert_eq!(expired.status(), StatusCode::FORBIDDEN);
    assert_eq!(body_of(expired).await, "This edit link has run out. Ask for a new one in Discord.");

    let token = bot.edit_grants.grant(EditGrant { thread_id: Id::new(1000), user_id: Id::new(4) });
    let response = get(&bot, &format!("/edit/{token}")).await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = url::Url::parse(header_of(&response, header::LOCATION)).unwrap();
    let query: Vec<(String, String)> = location.query_pairs().into_owned().collect();
    let data = &query.iter().find(|(name, _)| name == "data").unwrap().1;
    assert_eq!(serde_json::from_str::<Value>(data).unwrap()["name"]["und"], "Bar meet");
    assert!(query.contains(&("return".to_owned(), "https://p.test/edit/return".to_owned())));
    assert!(query.contains(&("state".to_owned(), token)));
}

#[tokio::test]
async fn edit_return_without_a_grant_is_refused() {
    let bot = seeded_bot();
    let page = get(&bot, "/edit/return").await;
    assert_eq!(page.status(), StatusCode::OK);
    assert!(body_of(page).await.contains("Back to the post"));
    let request = Request::post("/edit/return").body(Body::from("{\"state\":\"nope\"}")).unwrap();
    let response = send(&bot, request).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let answer: Value = serde_json::from_str(&body_of(response).await).unwrap();
    assert_eq!(answer, json!({ "message": "This edit link has run out. Ask for a new one in Discord." }));
}

#[tokio::test]
async fn lexend_is_served_as_woff2_with_a_long_cache() {
    let bot = seeded_bot();
    let response = get(&bot, "/fonts/lexend.woff2").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header_of(&response, header::CONTENT_TYPE), "font/woff2");
    assert_eq!(header_of(&response, header::CACHE_CONTROL), "public, max-age=31536000, immutable");
    assert!(response.into_body().collect().await.unwrap().to_bytes().starts_with(b"wOF2"));
}

#[tokio::test]
async fn config_link_without_a_grant_is_refused() {
    let bot = seeded_bot();
    assert_eq!(get(&bot, "/config/nope").await.status(), StatusCode::FORBIDDEN);
    let request = Request::post("/config/nope").body(Body::from("event=20")).unwrap();
    assert_eq!(send(&bot, request).await.status(), StatusCode::FORBIDDEN);
}

#[test]
fn config_form_keeps_only_single_values_per_forum() {
    let change = guild_config_change_of_form(b"event=20&event=21&announce%3A20=51&review%3A20=&review%3A20=30&other=1");
    assert_eq!(change.event_forum_ids, ["20".to_owned(), "21".to_owned()].into());
    assert_eq!(change.announce_role_id_of, [("20".to_owned(), "51".to_owned())].into());
    assert!(change.review_channel_id_of.is_empty());
}
