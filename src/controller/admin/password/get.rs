use axum::{debug_handler, extract::State, response::Response};
use axum_messages::Messages;
use axum_session::Session;
use axum_session_redispool::SessionRedisPool;
use serde_json::json;
use uuid::Uuid;

use crate::{controller::format, startup::AppState, Result};

#[debug_handler]
pub async fn change_password_form(
    messages: Messages,
    session: Session<SessionRedisPool>,
    State(state): State<AppState>,
) -> Result<Response> {
    if session.get::<Uuid>("user_id").is_none() {
        return format::render().redirect("/login");
    }
    let message = messages
        .into_iter()
        .map(|msg| format!("{}", msg))
        .collect::<Vec<_>>()
        .join(",");

    format::render().view(
        &state.tera_engine,
        "admin/password.html",
        json!({"message": message}),
    )
}
