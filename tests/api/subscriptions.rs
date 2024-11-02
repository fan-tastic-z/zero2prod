use axum::{
    body::Body,
    http::{self, Request},
    Router,
};
use tower::ServiceExt;
use zero2prod::startup::app;

use crate::helpers::spawn_app;

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
    let state = spawn_app().await;
    let app = app(state.clone());
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";
    let response = post_subscriptions(app, body).await;

    assert!(response.status().is_success());

    #[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
    struct Subscription {
        name: String,
        email: String,
    }
    let saved: Subscription = sqlx::query_as("SELECT email, name FROM subscriptions")
        .fetch_one(state.db_pool.as_ref())
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "fantastic.fun.zf@gmail.com");
    assert_eq!(saved.name, "fan-tastic.z");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let state = spawn_app().await;
    let app = app(state);
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
    let state = spawn_app().await;
    let app = app(state);

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
