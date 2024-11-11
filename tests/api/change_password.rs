use uuid::Uuid;

use crate::helpers::{assert_response_redirect_to, spawn_app};

#[tokio::test]
pub async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let test_app = spawn_app().await;
    let response = test_app.get_change_password_with_cookie("").await;
    assert_response_redirect_to(response, "/login");
}

#[tokio::test]
pub async fn new_password_fields_must_match() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();
    let cookie = test_app.login_and_get_cookie().await;
    let body = serde_json::json!({
        "current_password":test_app.test_user.password,
        "new_password": new_password,
        "new_password_check": another_new_password,
    });
    let response = test_app
        .post_update_password_with_cookie(body, &cookie)
        .await;

    assert_response_redirect_to(response, "/admin/password");
}

#[tokio::test]
pub async fn current_password_must_be_valid() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();
    let cookie = test_app.login_and_get_cookie().await;
    let body = serde_json::json!({
        "current_password": wrong_password,
        "new_password": new_password,
        "new_password_check": new_password,
    });
    let response = test_app
        .post_update_password_with_cookie(body, &cookie)
        .await;
    assert_response_redirect_to(response, "/admin/password");
}
