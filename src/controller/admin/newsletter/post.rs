use axum::{debug_handler, extract::State, response::Response, Extension, Form};
use axum_messages::Messages;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    controller::format,
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

    let mut transaction = match try_processing(&state.db_pool, &idempotency_key, user_id).await? {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            messages.info("The newsletter issue has been published!");
            return Ok(response);
        }
    };
    let issue_id = insert_newsletter_issue(
        &mut transaction,
        &params.title,
        &params.text_content,
        &params.html_content,
    )
    .await?;

    enqueue_delivery_tasks(&mut transaction, issue_id).await?;

    let response = format::render().redirect("/admin/newsletters")?;
    let response = save_response(transaction, &idempotency_key, user_id, response).await?;
    messages.info("The newsletter issue has been published!");
    Ok(response)
}

async fn insert_newsletter_issue(
    transaction: &mut Transaction<'static, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid> {
    let newsletter_issue_id = Uuid::new_v4();
    let query = sqlx::query(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
    )
    .bind(newsletter_issue_id)
    .bind(title)
    .bind(text_content)
    .bind(html_content);
    transaction.execute(query).await?;
    Ok(newsletter_issue_id)
}

async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'static, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<()> {
    let query = sqlx::query(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .bind(newsletter_issue_id);

    transaction.execute(query).await?;
    Ok(())
}
