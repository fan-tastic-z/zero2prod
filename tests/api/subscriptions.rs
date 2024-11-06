use axum::{
    body::Body,
    http::{self, Request},
    Router,
};
use tower::ServiceExt;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};
use zero2prod::startup::app;

use crate::helpers::{path_and_query, spawn_app, ConfirmationLinks, TestApp};

#[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
struct Subscription {
    name: String,
    email: String,
    status: String,
}

pub async fn post_subscriptions(app: Router, body: &str) -> http::Response<Body> {
    app.oneshot(
        Request::builder()
            .method(http::Method::POST)
            .uri("/subscriptions")
            .header(
                http::header::CONTENT_TYPE,
                mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
            )
            .body(Body::new(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state.clone());
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = post_subscriptions(app, body).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state.clone());
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    post_subscriptions(app, body).await;
    let saved: Subscription = sqlx::query_as("SELECT email, name, status FROM subscriptions")
        .fetch_one(test_app.app_state.db_pool.as_ref())
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "fantastic.fun.zf@gmail.com");
    assert_eq!(saved.name, "fan-tastic.z");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state);
    let test_cases = vec![
        ("name=fan-tastic.z", "missing the email"),
        ("email=fantastic.fun.zf@gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = post_subscriptions(app.clone(), invalid_body).await;
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state);

    let test_cases = vec![
        ("name=&email=fantastic.fun.zf@gmail.com", "empty name"),
        ("name=fan-tastic.z&email=", "empty email"),
        (
            "name=fan-tastic.z&email=definitely-not-an-email",
            "invalid email",
        ),
    ];
    for (body, description) in test_cases {
        let response = post_subscriptions(app.clone(), body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {}",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state);
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;
    let response = post_subscriptions(app, body).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state);
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;
    post_subscriptions(app, body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };
    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());
    assert_eq!(html_link, text_link);
}

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state);

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::GET)
                .uri("/subscriptions/confirm")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let test_app = spawn_app().await;
    let app_state = &test_app.app_state;
    let app = app(app_state.clone());
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;
    post_subscriptions(app.clone(), body).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    let path_and_query = path_and_query(confirmation_links.html);

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::GET)
                .uri(path_and_query)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let test_app = spawn_app().await;
    let app_state = &test_app.app_state;
    let app = app(app_state.clone());
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;
    post_subscriptions(app.clone(), body).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);
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

    let saved: Subscription = sqlx::query_as("SELECT email, name, status FROM subscriptions")
        .fetch_one(test_app.app_state.db_pool.as_ref())
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "fantastic.fun.zf@gmail.com");
    assert_eq!(saved.name, "fan-tastic.z");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn subscribe_fail_if_there_is_a_fatal_database_error() {
    let test_app = spawn_app().await;
    let app_state = &test_app.app_state;
    let app = app(app_state.clone());
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    sqlx::query("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;")
        .execute(app_state.db_pool.as_ref())
        .await
        .unwrap();
    let response = post_subscriptions(app, body).await;
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    let app_state = &test_app.app_state;
    let app = app(app_state.clone());
    create_unconfirmed_subscriber(app.clone(), &test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "</p>Newsletter body as HTML</p>"
        }
    });
    let response = post_newsletters(app, newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 200);
}

async fn create_unconfirmed_subscriber(app: Router, test_app: &TestApp) -> ConfirmationLinks {
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

async fn create_confirmed_subscriber(app: Router, test_app: &TestApp) {
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

pub async fn post_newsletters(app: Router, body: serde_json::Value) -> http::Response<Body> {
    app.oneshot(
        Request::builder()
            .method(http::Method::POST)
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .uri("/newsletters")
            .body(Body::new(body.to_string()))
            .unwrap(),
    )
    .await
    .expect("Failed to execute request newsletters.")
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    let app_state = &test_app.app_state;
    let app = app(app_state.clone());
    create_confirmed_subscriber(app.clone(), &test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body: serde_json::Value = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "</p>Newsletter body as HTML</p>"
        }
    });
    let response = post_newsletters(app, newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_422_for_invalid_data() {
    let test_app = spawn_app().await;
    let app_state = &test_app.app_state;
    let app = app(app_state.clone());

    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "</p>Newsletter body as HTML</p>"
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!"
            }),
            "missing content",
        ),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = post_newsletters(app.clone(), invalid_body).await;
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}
