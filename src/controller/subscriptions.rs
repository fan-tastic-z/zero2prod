use axum::{
    debug_handler,
    extract::{Form, State},
    response::Response,
};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    errors,
    startup::AppState,
    Result,
};

use super::format;

#[derive(Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[debug_handler]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(params): Form<FormData>,
) -> Result<Response> {
    let new_subscriber = params.try_into()?;

    let mut transaction = state.db_pool.begin().await?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber).await?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token).await?;

    transaction.commit().await?;

    send_confirm_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await?;
    format::empty()
}

pub async fn send_confirm_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<()> {
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
        .await?;
    Ok(())
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
    type Error = errors::Error;

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
) -> Result<()> {
    let query = sqlx::query(
        r#"
    INSERT INTO subscription_tokens (subscription_token,subscriber_id)
    VALUES ($1, $2)
    "#,
    )
    .bind(subscription_token)
    .bind(subscriber_id);
    transaction.execute(query).await?;
    Ok(())
}
