use wiremock::{matchers::any, Mock, ResponseTemplate};

use crate::helpers::{
    assert_response_redirect_to, create_confirmed_subscriber, create_unconfirmed_subscriber,
    spawn_app,
};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    let app = test_app.app().await;

    create_unconfirmed_subscriber(app.clone(), &test_app).await;

    let cookie = test_app.login_and_get_cookie().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });
    let response = test_app
        .post_newsletter_with_cookie(newsletter_request_body, &cookie)
        .await;
    assert_response_redirect_to(response, "/admin/newsletters");
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    let app = test_app.app().await;
    create_confirmed_subscriber(app.clone(), &test_app).await;
    let cookie = test_app.login_and_get_cookie().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });
    let response = test_app
        .post_newsletter_with_cookie(newsletter_request_body, &cookie)
        .await;
    assert_response_redirect_to(response, "/admin/newsletters");
}

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_newsletter_form() {
    let test_app = spawn_app().await;
    let response = test_app.get_publish_newsletter().await;
    assert_response_redirect_to(response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_a_newsletter() {
    let test_app = spawn_app().await;
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });
    let response = test_app
        .post_newsletter_with_cookie(newsletter_request_body, "")
        .await;
    assert_response_redirect_to(response, "/login");
}
