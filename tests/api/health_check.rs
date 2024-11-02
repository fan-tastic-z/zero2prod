use axum::{
    body::Body,
    http::{HeaderValue, Request},
};
use reqwest::header::CONTENT_LENGTH;
use tower::ServiceExt;
use zero2prod::startup::app;

use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;
    let app = app(test_app.app_state);

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
