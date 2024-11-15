use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use axum::{
    body::Body,
    http::{self, HeaderValue, Request},
    Router,
};
use fake::{
    faker::{internet::en::SafeEmail, name::en::Name},
    Fake,
};
use http_body_util::BodyExt;
use hyper::{
    header::{self, LOCATION},
    Response,
};
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
    configuration::{get_configuration, Settings},
    email_client::EmailClient,
    issue_delivery_worker::{try_execute_task, ExecutionOutcome},
    startup::{app, configuration_database, register_layer, AppState},
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
    pub configuration: Settings,
    pub email_client: EmailClient,
}

impl TestApp {
    pub async fn app(&self) -> Router {
        let app_state = self.app_state.clone();
        let router = app(app_state);
        self.register_layer(router).await
    }

    pub async fn register_layer(&self, app: Router) -> Router {
        register_layer(app, &self.configuration).await
    }

    pub async fn post_login(&self, body: serde_json::Value) -> http::Response<Body> {
        let body = serde_urlencoded::to_string(body).unwrap();
        self.app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .uri("/login")
                    .body(Body::new(body.to_string()))
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request login.")
    }

    pub async fn post_update_password_with_cookie(
        &self,
        body: serde_json::Value,
        cookie: &str,
    ) -> http::Response<Body> {
        let body = serde_urlencoded::to_string(body).unwrap();
        self.app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .uri("/admin/password")
                    .header(header::COOKIE, cookie)
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request change password.")
    }

    pub async fn post_logout_with_cookie(&self, cookie: &str) -> http::Response<Body> {
        self.app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .header(header::COOKIE, cookie)
                    .uri("/admin/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("failed to execute request logout.")
    }

    pub async fn get_admin_dashboard_with_cookie(&self, cookie: &str) -> http::Response<Body> {
        self.app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .header(header::COOKIE, cookie)
                    .uri("/admin/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request admin dashboard.")
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

    pub async fn post_newsletter_with_cookie(
        &self,
        body: &serde_json::Value,
        cookie: &str,
    ) -> http::Response<Body> {
        let body = serde_urlencoded::to_string(body).unwrap();
        self.app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .header(header::COOKIE, cookie)
                    .uri("/admin/newsletters")
                    .body(Body::new(body.to_string()))
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request newsletters.")
    }

    pub async fn get_publish_newsletter(&self) -> http::Response<Body> {
        self.app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .uri("/admin/newsletters")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request newsletters.")
    }

    pub async fn _get_publish_newsletter_html(&self, cookie: &str) -> String {
        let response = self
            .app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .header(header::COOKIE, cookie)
                    .uri("/admin/newsletters")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request newsletters.");
        // get response body string
        let body = response
            .into_body()
            .collect()
            .await
            .expect("Failed to collect body");
        String::from_utf8(body.to_bytes().to_vec()).expect("Failed to parse body to string")
    }

    pub async fn get_change_password_with_cookie(&self, cookie: &str) -> http::Response<Body> {
        self.app()
            .await
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .header(header::COOKIE, cookie)
                    .uri("/admin/password")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("Failed to execute request change password form.")
    }

    pub async fn login_and_get_cookie(&self) -> String {
        let body = serde_json::json!({
            "username":self.test_user.username,
            "password": self.test_user.password,
        });
        let response = self.post_login(body).await;
        get_cookie(response)
    }

    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.app_state.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
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
        // TODO: remove this
        configuration: configuration.clone(),
        email_client: configuration.email_client.client(),
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
            password: "everythinghastostartsomewhere".into(),
        }
    }

    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        dbg!(&password_hash);
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
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;
    post_subscriptions(app, &body).await;
    let email_request = test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    test_app.get_confirmation_links(&email_request)
}

pub fn assert_response_redirect_to(response: Response<Body>, to: &str) {
    let location = response.headers().get(LOCATION);
    assert_eq!(location, Some(&HeaderValue::from_str(to).unwrap()));
}

pub fn get_cookie(response: Response<Body>) -> String {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find_map(|value| {
            value.to_str().ok().and_then(|cookie| {
                if cookie.starts_with("session=") {
                    Some(cookie.to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| "".to_string())
}
