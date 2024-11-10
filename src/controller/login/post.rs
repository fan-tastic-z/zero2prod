use axum::{debug_handler, extract::State, response::Response, Form};
use axum_messages::Messages;
use axum_session::Session;
use axum_session_redispool::SessionRedisPool;

use crate::{
    authentication::{validate_credentials, Credentials},
    controller::format,
    domain::LoginForm,
    startup::AppState,
    Result,
};

#[debug_handler]
pub async fn login(
    session: Session<SessionRedisPool>,
    messages: Messages,
    State(state): State<AppState>,
    Form(params): Form<LoginForm>,
) -> Result<Response> {
    let credentials = Credentials {
        username: params.username,
        password: params.password,
    };
    let res = validate_credentials(credentials, &state.db_pool).await;
    match res {
        Ok(user_id) => {
            session.renew();
            session.set("user_id", user_id);
            format::render().redirect("/admin/dashboard")
        }
        Err(e) => {
            messages.error(e.to_string());
            format::render().redirect("/login")
        }
    }
}
