use axum::{
    debug_handler,
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Deserialize;
use tracing::{error, info};
use uuid::Uuid;

use crate::startup::AppState;

#[derive(Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[debug_handler]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(params): Form<FormData>,
) -> Result<Response, StatusCode> {
    let res = sqlx::query(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
    )
    .bind(Uuid::new_v4())
    .bind(params.email)
    .bind(params.name)
    .bind(Utc::now())
    .execute(state.db_pool.as_ref())
    .await;
    match res {
        Ok(_) => {
            info!("New subscriber detail have been saved");
            Ok((StatusCode::OK).into_response())
        }
        Err(e) => {
            error!("Failed to execute query: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
