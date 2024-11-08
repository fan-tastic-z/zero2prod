use errors::Error;

pub mod authentication;
pub mod backtrace;
pub mod configuration;
pub mod controller;
pub mod domain;
pub mod email_client;
pub mod errors;
pub mod middleware;
pub mod startup;
pub mod telemetry;
pub mod view_engine;

/// Application results options list
pub type Result<T, E = Error> = std::result::Result<T, E>;
