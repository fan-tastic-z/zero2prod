use crate::Result;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

#[allow(unused)]
pub fn json<T: Serialize>(t: T) -> Result<Response> {
    Ok(Json(t).into_response())
}

pub fn empty() -> Result<Response> {
    Ok(().into_response())
}

#[allow(unused)]
pub fn text(t: &str) -> Result<Response> {
    Ok(t.to_string().into_response())
}

#[allow(unused)]
pub fn empty_json() -> Result<Response> {
    json(json!({}))
}
