use std::sync::Arc;

use axum::{
    body::Body,
    http::{self, header::CONTENT_LENGTH, HeaderValue, Request},
};

use once_cell::sync::Lazy;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use zero2prod::{
    configuration::get_configuration,
    startup::{app, configuration_database, AppState},
    telemetry::init_tracing,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    init_tracing();
});

#[tokio::test]
async fn health_check_works() {
    let db_pool = Arc::new(spawn_app().await);
    let state = AppState { db_pool };
    let app = app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response.headers().get(CONTENT_LENGTH),
        Some(&HeaderValue::from_str("0").unwrap())
    );
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let db_pool = Arc::new(spawn_app().await);
    let state = AppState {
        db_pool: db_pool.clone(),
    };
    let app = app(state);
    let body = "name=fan-tastic.z&email=fantastic.fun.zf@gmail.com";

    let response = app
        .oneshot(
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
        .unwrap();

    assert!(response.status().is_success());

    #[derive(sqlx::FromRow, Debug, PartialEq, Eq)]
    struct Subscription {
        name: String,
        email: String,
    }
    let saved: Subscription = sqlx::query_as("SELECT email, name FROM subscriptions")
        .fetch_one(db_pool.as_ref())
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "fantastic.fun.zf@gmail.com");
    assert_eq!(saved.name, "fan-tastic.z");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let db_pool = Arc::new(spawn_app().await);
    let state = AppState { db_pool };
    let app = app(state);
    let test_cases = vec![
        ("name=fan-tastic.z", "missing the email"),
        ("email=fantastic.fun.zf@gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/subscriptions")
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .body(Body::new(invalid_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
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
    let db_pool = Arc::new(spawn_app().await);
    let state = AppState { db_pool };
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
        let response = app
            .clone()
            .oneshot(
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
            .unwrap();
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {}",
            description
        );
    }
}

async fn spawn_app() -> PgPool {
    Lazy::force(&TRACING);
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configuration_database(&configuration.database).await
}
