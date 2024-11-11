use secrecy::Secret;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    pub current_password: Secret<String>,
    pub new_password: Secret<String>,
    pub new_password_check: Secret<String>,
}
