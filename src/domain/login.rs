use secrecy::Secret;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: Secret<String>,
}
