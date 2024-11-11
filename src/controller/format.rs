use crate::{view_engine::TeraView, Result};
use axum::{
    body::Body,
    http::{response::Builder, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use hyper::{header, StatusCode};
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

#[must_use]
pub fn render() -> RenderBuilder {
    RenderBuilder::new()
}

#[derive(Debug, Default)]
pub struct RenderBuilder {
    response: Builder,
}

impl RenderBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            response: Builder::default(),
        }
    }

    pub fn view<S>(self, v: &TeraView, key: &str, data: S) -> Result<Response>
    where
        S: Serialize,
    {
        let content = v.render(key, data)?;
        self.html(&content)
    }

    pub fn html(self, content: &str) -> Result<Response> {
        Ok(self
            .response
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
            )
            .body(Body::from(content.to_string()))?)
    }

    pub fn redirect(self, to: &str) -> Result<Response> {
        Ok(self
            .response
            .status(StatusCode::SEE_OTHER)
            .header(header::LOCATION, to)
            .body(Body::empty())?)
    }
}
