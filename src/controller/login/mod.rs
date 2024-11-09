mod post;
use axum::{debug_handler, extract::State, response::Response};
use axum_messages::Messages;
pub use post::*;
use serde_json::json;

use crate::{startup::AppState, Result};

use super::format;

#[debug_handler]
pub async fn login_form(messages: Messages, State(state): State<AppState>) -> Result<Response> {
    let message = messages
        .into_iter()
        .map(|msg| format!("{}", msg))
        .collect::<Vec<_>>()
        .join(",");
    format::render().view(
        &state.tera_engine,
        "login.html",
        json!({"message": message}),
    )
}
