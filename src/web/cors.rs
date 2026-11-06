use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

const OPEN_EVNT_PREFIXES: [&str; 3] = ["/events", "/forums", "/calendars"];

fn is_open_evnt_path(path: &str) -> bool {
    OPEN_EVNT_PREFIXES
        .iter()
        .any(|prefix| path.strip_prefix(prefix).is_some_and(|rest| rest.is_empty() || rest.starts_with('/')))
}

fn add_cors_headers(headers: &mut HeaderMap) {
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, HeaderValue::from_static("ETag"));
}

pub async fn open_evnt_cors(request: Request, next: Next) -> Response {
    if !is_open_evnt_path(request.uri().path()) {
        return next.run(request).await;
    }
    if request.method() == Method::OPTIONS {
        let mut headers = HeaderMap::new();
        add_cors_headers(&mut headers);
        headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET"));
        headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("If-None-Match"));
        headers.insert(header::VARY, HeaderValue::from_static("Access-Control-Request-Headers"));
        return (StatusCode::NO_CONTENT, headers).into_response();
    }
    let mut response = next.run(request).await;
    add_cors_headers(response.headers_mut());
    response
}

#[cfg(test)]
mod tests {
    use super::is_open_evnt_path;

    #[test]
    fn open_evnt_paths_are_the_prefix_or_below_it() {
        assert!(is_open_evnt_path("/events"));
        assert!(is_open_evnt_path("/events/"));
        assert!(is_open_evnt_path("/calendars/a/events/1.evnt.json"));
        assert!(!is_open_evnt_path("/forumsx"));
        assert!(!is_open_evnt_path("/config/x"));
    }
}
