use crate::helpers::{assert_response_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let test_app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password",
    });
    let response: hyper::Response<axum::body::Body> = test_app.post_login(login_body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_response_redirect_to(response, "/login");
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let test_app = spawn_app().await;

    let body = serde_json::json!({
        "username":test_app.test_user.username,
        "password": test_app.test_user.password,
    });
    let response = test_app.post_login(body).await;
    assert_response_redirect_to(response, "/admin/dashboard");
}

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let test_app = spawn_app().await;
    let response = test_app.get_admin_dashboard_with_cookie("").await;
    assert_eq!(response.status().as_u16(), 303);
    assert_response_redirect_to(response, "/login");
}
