use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    async_trait, debug_handler,
    extract::{FromRequestParts, State},
    http::request::Parts,
    response::Response,
    Json, RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use base64::prelude::*;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::SubscriberEmail, errors::Error, startup::AppState, Result as zero2prodResult};

use super::format;

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[debug_handler]
pub async fn publish_newsletter(
    credentials: Credentials,
    State(state): State<AppState>,
    Json(params): Json<BodyData>,
) -> zero2prodResult<Response> {
    let _user_id = validate_credentials(credentials, &state.db_pool).await?;

    let subscribers = get_confirmed_subscribers(&state.db_pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(
                        subscriber.email,
                        &params.title,
                        &params.content.html,
                        &params.content.text,
                    )
                    .await?;
            }
            Err(_) => {
                tracing::warn!(
                    "Skipping a confirmed subscriber. \
                Their stored contact detail are invalid"
                )
            }
        }
    }
    format::empty()
}

#[derive(sqlx::FromRow, Debug)]
struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> zero2prodResult<Vec<zero2prodResult<ConfirmedSubscriber>>> {
    #[derive(sqlx::FromRow, Debug)]
    struct Row {
        email: String,
    }
    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT email FROM subscriptions WHERE status = 'confirmed'
    "#,
    )
    .fetch_all(pool)
    .await?;
    let confirmed_subscribers: Vec<zero2prodResult<ConfirmedSubscriber>> = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(e) => Err(e),
        })
        .collect();
    Ok(confirmed_subscribers)
}

#[derive(Debug)]
pub struct Credentials {
    username: String,
    password: Secret<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for Credentials
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            match parts.extract::<TypedHeader<Authorization<Bearer>>>().await {
                Ok(header) => header,
                Err(_) => {
                    return Err(Error::Unauthorized(
                        "header credentials token error".to_string(),
                    ))
                }
            };

        let decoded_credentials =
            // TODO: bearer.token error need to be handled
            match String::from_utf8(BASE64_STANDARD_NO_PAD.decode(bearer.token())?) {
                Ok(credentials) => credentials,
                Err(_) => {
                    return Err(Error::Unauthorized(
                        "header credentials token error".to_string()
                    ))
                }
            };

        let mut credentials = decoded_credentials.splitn(2, ':');
        let username = match credentials.next() {
            Some(username) => username.to_string(),
            None => {
                return Err(Error::Unauthorized(
                    "header credentials token error".to_string(),
                ))
            }
        };

        let password = match credentials.next() {
            Some(password) => password.to_string(),
            None => {
                return Err(Error::Unauthorized(
                    "header credentials token error".to_string(),
                ))
            }
        };

        Ok(Credentials {
            username,
            password: password.into(),
        })
    }
}

async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> zero2prodResult<Uuid> {
    let row: Option<User> = sqlx::query_as(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(credentials.username)
    .fetch_optional(pool)
    .await?;

    let (expected_password_hash, user_id) = match row {
        Some(row) => (row.password_hash, row.user_id),
        None => {
            return Err(Error::Unauthorized(
                "Failed to query to retrieve stored credentials.".to_string(),
            ))
        }
    };
    let expected_password_hash = PasswordHash::new(&expected_password_hash)?;
    Argon2::default().verify_password(
        credentials.password.expose_secret().as_bytes(),
        &expected_password_hash,
    )?;

    Ok(user_id)
}

#[derive(sqlx::FromRow)]
struct User {
    user_id: Uuid,
    password_hash: String,
}
