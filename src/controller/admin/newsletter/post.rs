use axum::{debug_handler, extract::State, response::Response, Extension, Form};
use axum_messages::Messages;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    controller::format,
    domain::SubscriberEmail,
    errors::Error,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    startup::AppState,
    Result,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[debug_handler]
pub async fn publish_newsletter(
    Extension(user_id): Extension<Uuid>,
    messages: Messages,
    State(state): State<AppState>,
    Form(params): Form<FormData>,
) -> Result<Response> {
    let idempotency_key: IdempotencyKey = params
        .idempotency_key
        .try_into()
        .map_err(|_| Error::InvalidIdempotencyKey)?;

    let transaction = match try_processing(&state.db_pool, &idempotency_key, user_id).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            messages.info("The newsletter issue has been published!");
            return Ok(response);
        }
    };
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
    let response = format::render().redirect("/admin/newsletters")?;
    let response = save_response(transaction, &idempotency_key, user_id, response).await?;
    Ok(response)
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
