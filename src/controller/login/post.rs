use axum::{debug_handler, extract::State, response::Response, Form};
use axum_messages::Messages;

use crate::{
    authentication::{validate_credentials, Credentials},
    controller::format,
    domain::LoginForm,
    startup::AppState,
    Result,
};

#[debug_handler]
pub async fn login(
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
        Ok(_) => format::render().redirect("/home"),
        Err(e) => {
            messages.error(e.to_string());
            format::render().redirect("/login")
        }
    }
}
