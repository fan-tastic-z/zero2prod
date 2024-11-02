use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::{postgres::PgPoolOptions, Connection};
use sqlx::{Executor, PgConnection, PgPool, Pool, Postgres};
use tower_http::trace::TraceLayer;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{health, subscribe},
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool<Postgres>>,
    pub email_client: Arc<EmailClient>,
}

impl AppState {
    pub async fn build(configuration: &Settings) -> Self {
        let db_pool = Arc::new(
            PgPoolOptions::new()
                .acquire_timeout(std::time::Duration::from_secs(2))
                .connect_lazy_with(configuration.database.with_db()),
        );

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let timeout = configuration.email_client.timeout();
        let email_client = Arc::new(EmailClient::new(
            configuration.email_client.base_url.clone(),
            sender_email,
            configuration.email_client.authorization_token.clone(),
            timeout,
        ));
        Self {
            db_pool,
            email_client,
        }
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/subscriptions", post(subscribe))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub async fn run_until_stopped(state: AppState, configuration: Settings) -> std::io::Result<()> {
    let app = app(state);
    let listener = tokio::net::TcpListener::bind(configuration.application.address()).await?;
    axum::serve(listener, app).await
}

pub async fn configuration_database(config: &DatabaseSettings) {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let db_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");
}
