use axum::{debug_handler, extract::State, response::Response, Form};
use axum_messages::Messages;
use axum_session::Session;
use axum_session_redispool::SessionRedisPool;
use secrecy::ExposeSecret;

use crate::{
    authentication::{change_password_store, validate_credentials, Credentials},
    controller::{admin::dashboard::get_username, format},
    domain::ChangePasswordForm,
    startup::AppState,
    Result,
};

#[debug_handler]
pub async fn change_password(
    session: Session<SessionRedisPool>,
    messages: Messages,
    State(state): State<AppState>,
    Form(params): Form<ChangePasswordForm>,
) -> Result<Response> {
    if params.new_password.expose_secret() != params.new_password_check.expose_secret() {
        messages.error("You entered two different new passwords - the field values must match.");
        return format::render().redirect("/admin/password");
    };
    let user_id = session.get("user_id");

    let user_id = match user_id {
        Some(user_id) => user_id,
        None => return format::render().redirect("/login"),
    };
    let username = get_username(user_id, &state.db_pool).await?;
    let credentials = Credentials {
        username,
        password: params.current_password,
    };
    if let Err(e) = validate_credentials(credentials, &state.db_pool).await {
        messages.error(e.to_string());
        return format::render().redirect("/admin/password");
    };
    change_password_store(user_id, params.new_password, &state.db_pool).await?;
    format::render().redirect("/admin/dashboard")
}
