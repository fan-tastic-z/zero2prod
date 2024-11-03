use anyhow::Context;
use axum::{
    debug_handler,
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::{Executor, Postgres, Transaction};
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
) -> Result<Response, SubscribeError> {
    let new_subscriber = params.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = state
        .db_pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    send_confirm_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;
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
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
    "#,
    )
    .bind(subscriber_id)
    .bind(new_subscriber.email.as_ref())
    .bind(new_subscriber.name.as_ref())
    .bind(Utc::now());
    transaction.execute(query).await?;
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
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query(
        r#"
    INSERT INTO subscription_tokens (subscription_token,subscriber_id)
    VALUES ($1, $2)
    "#,
    )
    .bind(subscription_token)
    .bind(subscriber_id);
    transaction.execute(query).await.map_err(StoreTokenError)?;
    Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
        trying to store a subscription token."
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("0")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> Response {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST.into_response(),
            SubscribeError::UnexpectedError(_cx) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
