use axum::{
    debug_handler,
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    startup::AppState,
};

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
    let new_subscriber = match params.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    match insert_subscriber(&state.db_pool, &new_subscriber).await {
        Ok(_) => Ok((StatusCode::OK).into_response()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
    )
    .bind(Uuid::new_v4())
    .bind(new_subscriber.email.as_ref())
    .bind(new_subscriber.name.as_ref())
    .bind(Utc::now())
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}
