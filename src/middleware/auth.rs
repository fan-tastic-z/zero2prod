use axum::{extract::Request, middleware::Next, response::Response};
use axum_session::Session;
use axum_session_redispool::SessionRedisPool;
use uuid::Uuid;

use crate::{controller::render, Result};

pub async fn auth_middleware(
    session: Session<SessionRedisPool>,
    request: Request,
    next: Next,
) -> Result<Response> {
    let user_id = session.get::<Uuid>("user_id");

    match user_id {
        Some(user_id) => user_id,
        None => return render().redirect("/login"),
    };
    Ok(next.run(request).await)
}
