use crate::helpers::{assert_response_redirect_to, spawn_app};

#[tokio::test]
async fn logout_clears_session_state() {
    let test_app = spawn_app().await;
    let cookie = test_app.login_and_get_cookie().await;
    let response = test_app.post_logout_with_cookie(&cookie).await;
    assert_response_redirect_to(response, "/login");
}
