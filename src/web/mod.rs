mod answers;
mod config_form;
mod config_page;
mod config_routes;
mod cors;
mod edit_return_page;
mod edit_routes;
mod font;
mod open_evnt_routes;
mod page;
#[cfg(test)]
mod cors_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::get,
};

pub use self::answers::{
    Answered, JSON_MEDIA_TYPE, OPEN_EVNT_MEDIA_TYPE, answer_unless_unchanged, html_answer, not_found, text_answer,
};
use crate::bot::Bot;

fn routes(bot: Arc<Bot>) -> Router {
    Router::new()
        .route("/events/{file}", get(open_evnt_routes::event_document))
        .route("/forums/{file}", get(open_evnt_routes::forum_index))
        .route("/calendars/{file}", get(open_evnt_routes::calendar_index))
        .route("/calendars/{token}/events/{file}", get(open_evnt_routes::calendar_document))
        .route("/edit/return", get(edit_routes::show_edit_return_page).post(edit_routes::save_edit_return))
        .route("/edit/{token}", get(edit_routes::open_editor))
        .route("/config/{token}", get(config_routes::show_config_page).post(config_routes::save_config))
        .route(font::LEXEND_PATH, get(font::serve_lexend))
        .fallback(|| async { not_found() })
        .method_not_allowed_fallback(|| async { not_found() })
        .with_state(bot)
}

async fn without_allow_header(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().remove(header::ALLOW);
    response
}

pub fn router(bot: Arc<Bot>) -> Router {
    Router::new()
        .fallback_service(routes(bot))
        .layer(middleware::from_fn(cors::open_evnt_cors))
        .layer(middleware::from_fn(without_allow_header))
}

pub async fn serve(bot: Arc<Bot>, host: &str, port: u16) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind((host, port)).await?;
    tracing::info!(host, port, public_url = %bot.links.public_url, "Listening");
    axum::serve(listener, router(bot)).await?;
    Ok(())
}
