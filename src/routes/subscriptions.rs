use axum::{
    debug_handler,
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
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
    let subscriber_id = match insert_subscriber(&state.db_pool, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let subscription_token = generate_subscription_token();
    if store_token(&state.db_pool, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if send_confirm_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };
    Ok((StatusCode::OK).into_response())
}

pub async fn send_confirm_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let plain_body = format!(
        "Welcome to our newsletter!<br /> \
            Click <a href=\"{}\">here</a> to confirm your subscription",
        confirmation_link
    );
    let html_body = &format!(
        "Welcome to our newsletter!<br /> \
            Click <a href=\"{}\">here</a> to confirm your subscription",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome!", html_body, &plain_body)
        .await
}

pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
    "#,
    )
    .bind(subscriber_id)
    .bind(new_subscriber.email.as_ref())
    .bind(new_subscriber.name.as_ref())
    .bind(Utc::now())
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

pub async fn store_token(
    pool: &PgPool,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
    INSERT INTO subscription_tokens (subscription_token,subscriber_id)
    VALUES ($1, $2)
    "#,
    )
    .bind(subscription_token)
    .bind(subscriber_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
