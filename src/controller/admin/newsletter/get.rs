use axum::{debug_handler, extract::State, response::Response};
use axum_messages::Messages;
use serde_json::json;

use crate::{controller::format, startup::AppState, Result};

#[debug_handler]
pub async fn publish_newsletter_form(
    messages: Messages,
    State(state): State<AppState>,
) -> Result<Response> {
    let messages = messages
        .into_iter()
        .map(|msg| format!("{}", msg))
        .collect::<Vec<_>>();

    format::render().view(
        &state.tera_engine,
        "admin/newsletter.html",
        json!({"messages": messages}),
    )
}
