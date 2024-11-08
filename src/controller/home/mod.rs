use axum::{debug_handler, extract::State, response::Response};
use serde_json::json;

use crate::{startup::AppState, Result};

use super::format;

#[debug_handler]
pub async fn home(State(state): State<AppState>) -> Result<Response> {
    format::render().view(&state.tera_engine, "home.html", json!({}))
}
