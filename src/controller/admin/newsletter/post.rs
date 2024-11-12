use axum::{debug_handler, extract::State, response::Response, Form};
use axum_messages::Messages;
use sqlx::PgPool;

use crate::{controller::format, domain::SubscriberEmail, startup::AppState, Result};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
}

#[debug_handler]
pub async fn publish_newsletter(
    messages: Messages,
    State(state): State<AppState>,
    Form(params): Form<FormData>,
) -> Result<Response> {
    let subscribers = get_confirmed_subscribers(&state.db_pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(
                        subscriber.email,
                        &params.title,
                        &params.html_content,
                        &params.text_content,
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
    messages.info("The newsletter issue has been published!");
    format::render().redirect("/admin/newsletters")
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
