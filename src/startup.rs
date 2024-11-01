use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::Connection;
use sqlx::{Executor, PgConnection, PgPool, Pool, Postgres};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{
    configuration::DatabaseSettings,
    email_client::EmailClient,
    routes::{health, subscribe},
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool<Postgres>>,
    pub email_client: Arc<EmailClient>,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/subscriptions", post(subscribe))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

pub async fn run(
    listener: TcpListener,
    db_pool: Arc<Pool<Postgres>>,
    email_client: Arc<EmailClient>,
) -> std::io::Result<()> {
    let state = AppState {
        db_pool,
        email_client,
    };
    let app = app(state);
    axum::serve(listener, app).await
}

pub async fn configuration_database(config: &DatabaseSettings) -> PgPool {
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
    db_pool
}
