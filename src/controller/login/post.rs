use axum::{debug_handler, extract::State, response::Response, Form};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

use crate::{
    authentication::{validate_credentials, Credentials},
    controller::format,
    domain::LoginForm,
    startup::AppState,
    Result,
};

#[debug_handler]
pub async fn login(
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
            let msg = format!("error={}", e);
            let hmac_tag = {
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                    state.hmac_secret.expose_secret().as_bytes(),
                )
                .unwrap();

                mac.update(msg.as_bytes());
                mac.finalize().into_bytes()
            };

            format::render().redirect_with_error("/login", &format!("{msg}&tag={hmac_tag:x}"))
        }
    }
}
