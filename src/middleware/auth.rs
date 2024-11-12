use axum::{extract::Request, middleware::Next, response::Response};
use axum_session::Session;
use axum_session_redispool::SessionRedisPool;
use uuid::Uuid;

use crate::{controller::render, Result};

#[derive(Clone)]
pub struct UserId(pub Uuid);

pub async fn auth_middleware(
    session: Session<SessionRedisPool>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let user_id = session.get::<Uuid>("user_id");

    let user_id = match user_id {
        Some(user_id) => user_id,
        None => return render().redirect("/login"),
    };
    request.extensions_mut().insert(user_id);
    Ok(next.run(request).await)
}
