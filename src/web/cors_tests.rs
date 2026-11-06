use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};

use super::tests::{body_of, header_of, seeded_bot, send};

#[tokio::test]
async fn preflight_is_answered_with_no_content() {
    let bot = seeded_bot();
    let request = Request::options("/events/1.evnt.json")
        .header(header::ORIGIN, "https://x.test")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .body(Body::empty())
        .unwrap();
    let response = send(&bot, request).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN), "*");
    assert_eq!(header_of(&response, header::ACCESS_CONTROL_ALLOW_METHODS), "GET");
    assert_eq!(header_of(&response, header::ACCESS_CONTROL_ALLOW_HEADERS), "If-None-Match");
    assert_eq!(header_of(&response, header::VARY), "Access-Control-Request-Headers");
    assert!(response.headers().get(header::ALLOW).is_none());
}

#[tokio::test]
async fn unknown_open_evnt_paths_still_carry_cors_headers() {
    let bot = seeded_bot();
    let response = send(&bot, Request::get("/events/").body(Body::empty()).unwrap()).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN), "*");
    assert_eq!(header_of(&response, header::ACCESS_CONTROL_EXPOSE_HEADERS), "ETag");
    let elsewhere = send(&bot, Request::get("/forumsx").body(Body::empty()).unwrap()).await;
    assert!(elsewhere.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}

#[tokio::test]
async fn unrouted_methods_are_not_found_like_hono() {
    let bot = seeded_bot();
    let response = send(&bot, Request::put("/edit/return").body(Body::empty()).unwrap()).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(header_of(&response, header::CONTENT_TYPE), "text/plain; charset=UTF-8");
    assert!(response.headers().get(header::ALLOW).is_none());
    assert_eq!(body_of(response).await, "404 Not Found");
}
