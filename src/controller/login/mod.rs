mod post;
use axum::{
    debug_handler,
    extract::{Query, State},
    response::Response,
};
use hmac::{Hmac, Mac};
pub use post::*;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_json::json;

use crate::{startup::AppState, Result};

use super::format;

#[derive(Deserialize)]
pub struct QueryPrams {
    error: String,
    tag: String,
}

impl QueryPrams {
    fn verify(self, secret: &Secret<String>) -> Result<String> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", self.error);
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        Ok(self.error)
    }
}

#[debug_handler]
pub async fn login_form(
    State(state): State<AppState>,
    params: Option<Query<QueryPrams>>,
) -> Result<Response> {
    let message = match params {
        Some(Query(p)) => match p.verify(&state.hmac_secret) {
            Ok(message) => Some(message),
            Err(_) => {
                tracing::warn!("Failed to verify mac");
                None
            }
        },
        None => None,
    };
    format::render().view(
        &state.tera_engine,
        "login.html",
        json!({"message": message}),
    )
}
