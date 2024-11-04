use std::sync::OnceLock;

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use regex::Regex;
use uuid::Uuid;

const X_REQUEST_ID: &str = "x-request-id";
const MAX_LEN: usize = 255;

static ID_CLEANUP: OnceLock<Regex> = OnceLock::new();

fn get_id_cleanup() -> &'static Regex {
    ID_CLEANUP.get_or_init(|| Regex::new(r"[^\w\-@]").unwrap())
}

#[derive(Debug, Clone)]
pub struct Zero2prodRequestId(String);

impl Zero2prodRequestId {
    #[must_use]
    pub fn get(&self) -> &str {
        self.0.as_str()
    }
}

pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let header_request_id = request.headers().get(X_REQUEST_ID).cloned();
    let request_id = make_request_id(header_request_id);
    request
        .extensions_mut()
        .insert(Zero2prodRequestId(request_id.clone()));
    let mut res = next.run(request).await;

    if let Ok(v) = HeaderValue::from_str(request_id.as_str()) {
        res.headers_mut().insert(X_REQUEST_ID, v);
    } else {
        tracing::warn!("could not set request ID into response headers: `{request_id}`",);
    }
    res
}

/// Generates or sanitizes a request ID.
fn make_request_id(maybe_request_id: Option<HeaderValue>) -> String {
    maybe_request_id
        .and_then(|hdr| {
            let id: Option<String> = hdr.to_str().ok().map(|s| {
                get_id_cleanup()
                    .replace_all(s, "")
                    .chars()
                    .take(MAX_LEN)
                    .collect()
            });
            id.filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}
