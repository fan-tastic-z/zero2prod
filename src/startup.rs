use std::sync::Arc;

use axum::{
    http,
    routing::{get, post},
    Router,
};
use axum_messages::MessagesManagerLayer;
use axum_session::{SessionConfig, SessionLayer, SessionStore};
use axum_session_redispool::SessionRedisPool;
use redis_pool::RedisPool;
use secrecy::ExposeSecret;
use sqlx::{postgres::PgPoolOptions, Connection};
use sqlx::{Executor, PgConnection, PgPool, Pool, Postgres};
use tower_http::trace::TraceLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::{
    configuration::{DatabaseSettings, Settings},
    controller::{
        admin_dashboard, change_password, change_password_form, confirm, health, home, login,
        login_form, logout, publish_newsletter, subscribe,
    },
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
        let tera_engine = Arc::new(TeraView::build().expect("Failed to init tera view engine"));
        Self {
            db_pool,
            email_client,
            base_url: configuration.application.base_url.clone(),
            tera_engine,
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
        .route("/admin/dashboard", get(admin_dashboard))
        .route("/admin/password", get(change_password_form))
        .route("/admin/password", post(change_password))
        .route("/admin/logout", post(logout))
        .with_state(state)
}

pub async fn run_until_stopped(state: AppState, configuration: Settings) -> Result<()> {
    let app = register_layer(app(state), &configuration).await;

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

async fn init_session_store(redis_url: &str) -> SessionStore<SessionRedisPool> {
    let client =
        redis::Client::open(redis_url).expect("Failed when trying to open the redis connection");
    let pool = RedisPool::from(client);
    let session_config = SessionConfig::default();
    SessionStore::<SessionRedisPool>::new(Some(pool.clone().into()), session_config)
        .await
        .expect("Failed to init session store")
}

pub async fn register_layer(app: Router, configuration: &Settings) -> Router {
    let session_store = init_session_store(configuration.redis_uri.expose_secret()).await;
    let memory_session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(memory_session_store).with_secure(false);

    app.layer(
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
    .layer(MessagesManagerLayer)
    .layer(SessionLayer::new(session_store))
    .layer(session_layer)
}
