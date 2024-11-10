use crate::{controller::format, startup::AppState, Result};
use axum::{debug_handler, extract::State, response::Response};
use axum_session::Session;
use axum_session_redispool::SessionRedisPool;
use serde_json::json;
use sqlx::{prelude::FromRow, PgPool};
use uuid::Uuid;

#[debug_handler]
pub async fn admin_dashboard(
    session: Session<SessionRedisPool>,
    State(state): State<AppState>,
) -> Result<Response> {
    let user_id = session.get("user_id");

    let username = match user_id {
        Some(user_id) => get_username(user_id, &state.db_pool).await?,
        None => return format::render().redirect("/login"),
    };

    format::render().view(
        &state.tera_engine,
        "admin/dashboard.html",
        json!({"username": username}),
    )
}

#[derive(FromRow)]
struct Username(String);

async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String> {
    let row: Username = sqlx::query_as(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
