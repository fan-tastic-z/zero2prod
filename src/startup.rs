use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    Executor, PgPool, Pool, Postgres,
};
use tokio::net::TcpListener;

use crate::{
    configuration::DatabaseSettings,
    routes::{health, subscribe},
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool<Postgres>>,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/subscriptions", post(subscribe))
        .with_state(state)
}

pub async fn run(listener: TcpListener, db_pool: Arc<Pool<Postgres>>) -> std::io::Result<()> {
    let state = AppState { db_pool };
    let app = app(state);
    axum::serve(listener, app).await
}

pub async fn configuration_database(config: &DatabaseSettings) -> PgPool {
    let options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .username(&config.username)
        .password(&config.password);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options.clone())
        .await
        .expect("Failed to connect to Postgres");

    pool.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");
    let new_options = options.database(&config.database_name);
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(new_options)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");
    db_pool
}
