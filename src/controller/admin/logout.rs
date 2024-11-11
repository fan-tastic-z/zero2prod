use axum::{debug_handler, response::Response};
use axum_messages::Messages;
use axum_session::Session;
use axum_session_redispool::SessionRedisPool;
use uuid::Uuid;

use crate::{controller::format, Result};

#[debug_handler]
pub async fn logout(messages: Messages, session: Session<SessionRedisPool>) -> Result<Response> {
    let user_id = session.get::<Uuid>("user_id");
    if user_id.is_none() {
        return format::render().redirect("/login");
    }
    session.destroy();
    messages.success("You have successfully logged out.");
    format::render().redirect("/login")
}
