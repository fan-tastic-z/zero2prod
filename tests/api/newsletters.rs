use axum::{
    body::Body,
    http::{self, Request},
};
use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine};
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{matchers::any, Mock, ResponseTemplate};

use crate::helpers::{create_confirmed_subscriber, spawn_app};

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    let app = test_app.app();
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
    let response = test_app.post_newsletters(newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_422_for_invalid_data() {
    let test_app = spawn_app().await;

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
        let response = test_app.post_newsletters(invalid_body).await;
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    let test_app = spawn_app().await;
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();
    let body: serde_json::Value = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "</p>Newsletter body as HTML</p>"
        }
    });
    let response = test_app
        .app()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .header(
                    http::header::AUTHORIZATION,
                    format!(
                        "Bearer {}",
                        BASE64_STANDARD_NO_PAD.encode(format!("{}:{}", username, password))
                    ),
                )
                .uri("/newsletters")
                .body(Body::new(body.to_string()))
                .unwrap(),
        )
        .await
        .expect("Failed to execute request newsletters.");
    assert_eq!(401, response.status().as_u16());
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    let test_app = spawn_app().await;
    let username = &test_app.test_user.username;
    let password = Uuid::new_v4().to_string();
    let body: serde_json::Value = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "</p>Newsletter body as HTML</p>"
        }
    });
    let response = test_app
        .app()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .header(
                    http::header::AUTHORIZATION,
                    format!(
                        "Bearer {}",
                        BASE64_STANDARD_NO_PAD.encode(format!("{}:{}", username, password))
                    ),
                )
                .uri("/newsletters")
                .body(Body::new(body.to_string()))
                .unwrap(),
        )
        .await
        .expect("Failed to execute request newsletters.");
    assert_eq!(401, response.status().as_u16());
}
