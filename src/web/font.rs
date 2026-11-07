use axum::{
    http::header,
    response::{IntoResponse, Response},
};

pub const LEXEND_PATH: &str = "/fonts/lexend.woff2";
const LEXEND_WOFF2: &[u8] = include_bytes!("../../assets/fonts/Lexend.woff2");

pub async fn serve_lexend() -> Response {
    let headers = [(header::CONTENT_TYPE, "font/woff2"), (header::CACHE_CONTROL, "public, max-age=31536000, immutable")];
    (headers, LEXEND_WOFF2).into_response()
}
