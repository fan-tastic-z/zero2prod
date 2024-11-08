use axum::{debug_handler, extract::State, response::Response, Json};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, Credentials},
    domain::SubscriberEmail,
    startup::AppState,
    Result,
};

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
) -> Result<Response> {
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

async fn get_confirmed_subscribers(pool: &PgPool) -> Result<Vec<Result<ConfirmedSubscriber>>> {
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
    let confirmed_subscribers: Vec<Result<ConfirmedSubscriber>> = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(e) => Err(e),
        })
        .collect();
    Ok(confirmed_subscribers)
}
