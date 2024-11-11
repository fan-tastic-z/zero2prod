use axum::{
    body::Body,
    http::{self, HeaderValue},
};
use hyper::{
    header::{self, LOCATION},
    Request,
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::helpers::spawn_app;

#[tokio::test]
pub async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_change_password().await;
    let location = response.headers().get(LOCATION);
    assert_eq!(location, Some(&HeaderValue::from_str("/login").unwrap()));
}

#[tokio::test]
pub async fn new_password_fields_must_match() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();
    let app = test_app.app().await;
    let body = serde_json::json!({
        "username":test_app.test_user.username,
        "password": test_app.test_user.password,
    });
    let body = serde_urlencoded::to_string(body).unwrap();
    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .header(
                    http::header::CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                )
                .uri("/login")
                .body(Body::new(body))
                .unwrap(),
        )
        .await
        .expect("Failed to execute request login.");
    let cookie = login_response
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
        .unwrap();
    let params = serde_json::json!({
        "current_password":test_app.test_user.password,
        "new_password": new_password,
        "new_password_check": another_new_password,
    });
    let body = serde_urlencoded::to_string(params).unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .header(
                    http::header::CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                )
                .uri("/admin/password")
                .header("cookie", cookie)
                .body(Body::new(body))
                .unwrap(),
        )
        .await
        .expect("Failed to execute request change password.");
    let location = response.headers().get(LOCATION);
    assert_eq!(
        location,
        Some(&HeaderValue::from_str("/admin/password").unwrap())
    );
}

#[tokio::test]
pub async fn current_password_must_be_valid() {
    let test_app = spawn_app().await;
    let app = test_app.app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    let body = serde_json::json!({
        "username":test_app.test_user.username,
        "password": test_app.test_user.password,
    });
    let body = serde_urlencoded::to_string(body).unwrap();
    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .header(
                    http::header::CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                )
                .uri("/login")
                .body(Body::new(body))
                .unwrap(),
        )
        .await
        .expect("failed to execute request login.");
    let cookie = login_response
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
        .unwrap();

    let change_password_form = serde_json::json!({
        "current_password": wrong_password,
        "new_password": new_password,
        "new_password_check": new_password,
    });
    let body = serde_urlencoded::to_string(change_password_form).unwrap();
    let response = &app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .header(
                    http::header::CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                )
                .header("cookie", cookie)
                .uri("/admin/password")
                .body(Body::new(body))
                .unwrap(),
        )
        .await
        .expect("Failed to execute request login.");
    let location = response.headers().get(LOCATION);
    assert_eq!(
        location,
        Some(&HeaderValue::from_str("/admin/password").unwrap())
    );
}
