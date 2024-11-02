use axum::{
    debug_handler,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::startup::AppState;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[debug_handler]
pub async fn confirm(
    State(state): State<AppState>,
    Query(params): Query<Parameters>,
) -> Result<Response, StatusCode> {
    let id = match get_subscriber_id_from_token(&state.db_pool, &params.subscription_token).await {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    dbg!(id);
    match id {
        Some(subscriber_id) => {
            if confirm_subscriber(&state.db_pool, subscriber_id)
                .await
                .is_err()
            {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Ok((StatusCode::OK).into_response())
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
            UPDATE subscriptions SET status = 'confirmed' WHERE id = $1
        "#,
    )
    .bind(subscriber_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct SubscriberId(Uuid);

async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result: Option<SubscriberId> = sqlx::query_as(
        r#"
        SELECT subscriber_id FROM subscription_tokens WHERE subscription_token=$1
        "#,
    )
    .bind(subscription_token)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.0))
}
