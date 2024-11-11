use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{async_trait, extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::Error, telemetry::spawn_blocking_with_tracing, Result};

#[derive(Debug)]
pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
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

pub async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Result<Uuid> {
    let row = get_stored_credentials(&credentials.username, pool).await?;

    let (expected_password_hash, user_id) = match row {
        Some(user) => (user.password_hash, user.user_id),
        None => {
            return Err(Error::Unauthorized(
                "Failed to query to retrieve stored credentials.".to_string(),
            ))
        }
    };
    let task_res = spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await;
    match task_res {
        Ok(verify_res) => {
            if verify_res.is_err() {
                return Err(Error::Unauthorized(
                    "Failed to verify password hash.".to_string(),
                ));
            }
        }
        Err(_) => {
            return Err(Error::Unauthorized(
                "Failed to verify password hash.".to_string(),
            ))
        }
    }

    Ok(user_id)
}

#[derive(sqlx::FromRow)]
struct User {
    user_id: Uuid,
    password_hash: String,
}

async fn get_stored_credentials(username: &str, pool: &PgPool) -> Result<Option<User>> {
    let row: Option<User> = sqlx::query_as(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

fn verify_password_hash(
    expected_password_hash: String,
    password_candidate: Secret<String>,
) -> Result<()> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash)?;
    Argon2::default().verify_password(
        password_candidate.expose_secret().as_bytes(),
        &expected_password_hash,
    )?;
    Ok(())
}

pub async fn change_password_store(
    user_id: Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> Result<()> {
    let password_hash =
        spawn_blocking_with_tracing(move || compute_password_hash(password)).await??;
    sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
    )
    .bind(password_hash.expose_secret())
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>> {
    let slat = SaltString::generate(&mut rand::thread_rng());
    let password = Argon2::default()
        .hash_password(password.expose_secret().as_bytes(), &slat)?
        .to_string();
    Ok(Secret::new(password))
}
