use axum::{
    body::Body,
    http::{self, HeaderValue},
};
use hyper::{
    header::{self, LOCATION},
    Request,
};
use tower::ServiceExt;

use crate::helpers::spawn_app;

#[tokio::test]
async fn logout_clears_session_state() {
    let test_app = spawn_app().await;
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
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .header(
                    http::header::CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                )
                .header("cookie", cookie)
                .uri("/admin/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("failed to execute request logout.");
    let location = response.headers().get(LOCATION);
    assert_eq!(location, Some(&HeaderValue::from_str("/login").unwrap()));
}
