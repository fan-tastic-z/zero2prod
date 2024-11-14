use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, prelude::FromRow, Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{configuration::Settings, domain::SubscriberEmail, email_client::EmailClient, Result};

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<()> {
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());
    let email_client = configuration.email_client.client();
    worker_loop(connection_pool, email_client).await
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> Result<()> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome> {
    let task = dequeue_task(pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }

    let (transaction, issue_id, email) = task.unwrap();
    tracing::info!("Delivering issue {} to {}", issue_id, email);
    match SubscriberEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = get_issue(pool, issue_id).await?;
            if let Err(e) = email_client
                .send_email(
                    email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to a confirmed subscriber. \
                        Skipping.",
                );
            }
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
            );
        }
    }
    delete_task(transaction, issue_id, &email).await?;
    Ok(ExecutionOutcome::TaskCompleted)
}

type PgTransaction = Transaction<'static, Postgres>;
async fn dequeue_task(pool: &PgPool) -> Result<Option<(PgTransaction, Uuid, String)>> {
    let mut transaction = pool.begin().await?;
    #[derive(FromRow)]
    struct Row {
        newsletter_issue_id: Uuid,
        subscriber_email: String,
    }
    let r: Option<Row> = sqlx::query_as(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *transaction)
    .await?;
    if let Some(r) = r {
        Ok(Some((
            transaction,
            r.newsletter_issue_id,
            r.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

async fn delete_task(mut transaction: PgTransaction, issue_id: Uuid, email: &str) -> Result<()> {
    let query = sqlx::query(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
    )
    .bind(issue_id)
    .bind(email);
    transaction.execute(query).await?;
    transaction.commit().await?;
    Ok(())
}

#[derive(FromRow)]
struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

async fn get_issue(pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue> {
    let issue: NewsletterIssue = sqlx::query_as(
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE
            newsletter_issue_id = $1
        "#,
    )
    .bind(issue_id)
    .fetch_one(pool)
    .await?;
    Ok(issue)
}
