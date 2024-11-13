use std::string;

use crate::{backtrace, Result};
use axum::{extract::FromRequest, http::StatusCode, response::IntoResponse};
use colored::Colorize;
use hyper::header::InvalidHeaderValue;
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{inner}\n{backtrace}")]
    WithBacktrace {
        inner: Box<Self>,
        backtrace: Box<std::backtrace::Backtrace>,
    },

    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    JSON(serde_json::Error),

    #[error(transparent)]
    Axum(#[from] axum::http::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),
    #[error(transparent)]
    FromUtf8(#[from] string::FromUtf8Error),
    #[error(transparent)]
    Argon2(#[from] argon2::Error),
    #[error(transparent)]
    Argon2PasswordHashError(#[from] argon2::password_hash::Error),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Tera(#[from] tera::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error(transparent)]
    InvalidStatusCode(#[from] axum::http::status::InvalidStatusCode),
    #[error(transparent)]
    AxumError(#[from] axum::Error),

    // API
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("internal server error")]
    InternalServerError,
    #[error("")]
    CustomError(StatusCode, ErrorDetail),
    #[error("")]
    InvalidIdempotencyKey,

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    pub fn wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Any(Box::new(err))
    }

    pub fn msg(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Message(err.to_string())
    }
    #[must_use]
    pub fn string(s: &str) -> Self {
        Self::Message(s.to_string())
    }
    #[must_use]
    pub fn bt(self) -> Self {
        let backtrace = std::backtrace::Backtrace::capture();
        match backtrace.status() {
            std::backtrace::BacktraceStatus::Disabled
            | std::backtrace::BacktraceStatus::Unsupported => self,
            _ => Self::WithBacktrace {
                inner: Box::new(self),
                backtrace: Box::new(backtrace),
            },
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(val: serde_json::Error) -> Self {
        Self::JSON(val).bt()
    }
}

pub fn bad_request<T: Into<String>, U>(msg: T) -> Result<U> {
    Err(Error::BadRequest(msg.into()))
}

#[derive(Debug, Serialize)]
/// Structure representing details about an error.
pub struct ErrorDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ErrorDetail {
    #[must_use]
    pub fn new<T: Into<String>>(error: T, description: T) -> Self {
        Self {
            error: Some(error.into()),
            description: Some(description.into()),
        }
    }
    #[must_use]
    pub fn with_reason<T: Into<String>>(error: T) -> Self {
        Self {
            error: Some(error.into()),
            description: None,
        }
    }
}

#[derive(Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(Error))]
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match &self {
            Self::WithBacktrace {
                inner,
                backtrace: _,
            } => {
                tracing::error!(
                error.msg = %inner,
                error.details = ?inner,
                "controller_error"
                );
            }
            err => {
                tracing::error!(
                error.msg = %err,
                error.details = ?err,
                "controller_error"
                );
            }
        }
        let public_facing_error = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                ErrorDetail::new("not_found", "Resource was not found"),
            ),
            Self::Unauthorized(err) => {
                tracing::warn!(err);
                (
                    StatusCode::UNAUTHORIZED,
                    ErrorDetail::new(
                        "unauthorized",
                        "You do not have permission to access this resource",
                    ),
                )
            }
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),

            Self::CustomError(status_code, data) => (status_code, data),
            Self::WithBacktrace { inner, backtrace } => {
                println!("\n{}", inner.to_string().red().underline());
                backtrace::print_backtrace(&backtrace).unwrap();
                (
                    StatusCode::BAD_REQUEST,
                    ErrorDetail::with_reason("Bad Request"),
                )
            }
            _ => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::with_reason("Bad Request"),
            ),
        };
        (public_facing_error.0, Json(public_facing_error.1)).into_response()
    }
}
