use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use axum::{
    body::Body,
    http::{self, Request},
    Router,
};
use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine};
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};
use zero2prod::{
    configuration::get_configuration,
    startup::{app, configuration_database, AppState},
    telemetry::init,
};

use crate::subscriptions::post_subscriptions;

static TRACING: Lazy<()> = Lazy::new(|| {
    let configuration = get_configuration().expect("Failed to read configuration.");
    init(&configuration.logger);
});

pub struct TestApp {
    pub app_state: AppState,
    pub email_server: MockServer,
    pub test_user: TestUser,
}

impl TestApp {
    pub fn app(&self) -> Router {
        let app_state = self.app_state.clone();
        app(app_state)
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link
        };
        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> http::Response<Body> {
        self.app()
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(
                        http::header::AUTHORIZATION,
                        format!(
                            "Bearer {}",
                            BASE64_STANDARD_NO_PAD.encode(format!(
                                "{}:{}",
                                self.test_user.username, self.test_user.password
                            ))
                        ),
                    )
                    .uri("/newsletters")
                    .body(Body::new(body.to_string()))
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request newsletters.")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configuration.email_client.base_url = email_server.uri();
    configuration_database(&configuration.database).await;
    let app_state = AppState::build(&configuration).await;

    let test_app = TestApp {
        app_state: app_state.clone(),
        email_server,
        test_user: TestUser::generate(),
    };
    test_app.test_user.store(&app_state.db_pool).await;
    test_app
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub fn path_and_query(link: reqwest::Url) -> String {
    let url_path = link.path();
    let query = link.query().unwrap();
    format!("{}?{}", url_path, query)
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        sqlx::query(
            r#"
            INSERT INTO users(user_id, username, password_hash)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(self.user_id)
        .bind(self.username.clone())
        .bind(password_hash)
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

pub async fn create_confirmed_subscriber(app: Router, test_app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app.clone(), test_app).await;
    let path_and_query = path_and_query(confirmation_links.plain_text);

    app.oneshot(
        Request::builder()
            .method(http::Method::GET)
            .uri(path_and_query)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
}

pub async fn create_unconfirmed_subscriber(app: Router, test_app: &TestApp) -> ConfirmationLinks {
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;
    post_subscriptions(app, body).await;
    let email_request = test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    test_app.get_confirmation_links(&email_request)
}
