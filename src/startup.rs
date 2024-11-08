use std::sync::Arc;

use axum::{
    http,
    routing::{get, post},
    Router,
};
use secrecy::Secret;
use sqlx::{postgres::PgPoolOptions, Connection};
use sqlx::{Executor, PgConnection, PgPool, Pool, Postgres};
use tower_http::trace::TraceLayer;

use crate::{
    configuration::{DatabaseSettings, Settings},
    controller::{confirm, health, home, login, login_form, publish_newsletter, subscribe},
    email_client::EmailClient,
    middleware::{request_id_middleware, Zero2prodRequestId},
    view_engine::TeraView,
    Result,
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool<Postgres>>,
    pub email_client: Arc<EmailClient>,
    pub base_url: String,
    pub tera_engine: Arc<TeraView>,
    pub hmac_secret: Secret<String>,
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
        let hmac_secret = configuration.application.hmac_secret.clone();
        let tera_engine = Arc::new(TeraView::build().expect("Failed to init tera view engine"));
        Self {
            db_pool,
            email_client,
            base_url: configuration.application.base_url.clone(),
            tera_engine,
            hmac_secret,
        }
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/home", get(home))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &http::Request<_>| {
                let ext = request.extensions();
                let request_id = ext
                    .get::<Zero2prodRequestId>()
                    .map_or_else(|| "req-id-none".to_string(), |r| r.get().to_string());
                let user_agent = request
                    .headers()
                    .get(axum::http::header::USER_AGENT)
                    .map_or("", |h| h.to_str().unwrap_or(""));

                tracing::error_span!(
                    "http-request",
                    "http.method" = tracing::field::display(request.method()),
                    "http.uri" = tracing::field::display(request.uri()),
                    "http.version" = tracing::field::debug(request.version()),
                    "http.user_agent" = tracing::field::display(user_agent),
                    request_id = tracing::field::display(request_id),
                )
            }),
        )
        .layer(axum::middleware::from_fn(request_id_middleware))
}

pub async fn run_until_stopped(state: AppState, configuration: Settings) -> Result<()> {
    let app = app(state);
    let listener = tokio::net::TcpListener::bind(configuration.application.address()).await?;
    axum::serve(listener, app).await?;
    Ok(())
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
