use axum::{
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::event::index::etag_of;

pub const OPEN_EVNT_MEDIA_TYPE: &str = "application/vnd.evnt+json";
pub const JSON_MEDIA_TYPE: &str = "application/json; charset=utf-8";
const TEXT_MEDIA_TYPE: &str = "text/plain; charset=UTF-8";
const HTML_MEDIA_TYPE: &str = "text/html; charset=UTF-8";

pub struct InternalError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for InternalError {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        tracing::error!(error = ?self.0, "HTTP request failed");
        text_answer(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    }
}

pub type Answered = Result<Response, InternalError>;

pub fn text_answer(status: StatusCode, text: &'static str) -> Response {
    (status, [(header::CONTENT_TYPE, TEXT_MEDIA_TYPE)], text).into_response()
}

pub fn html_answer(html: String) -> Response {
    ([(header::CONTENT_TYPE, HTML_MEDIA_TYPE)], html).into_response()
}

pub fn not_found() -> Response {
    text_answer(StatusCode::NOT_FOUND, "404 Not Found")
}

pub fn answer_unless_unchanged(request_headers: &HeaderMap, body: String, media_type: &'static str) -> Response {
    let etag = etag_of(&body);
    let known: Vec<&str> = request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map_or_default(|value| value.split(',').map(str::trim).collect());
    let unchanged = known.iter().any(|tag| *tag == etag || *tag == "*");
    let headers = [
        (header::ETAG, HeaderValue::from_str(&etag).expect("etag is ASCII")),
        (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
    ];
    if unchanged {
        return (StatusCode::NOT_MODIFIED, headers).into_response();
    }
    (headers, [(header::CONTENT_TYPE, media_type)], body).into_response()
}
