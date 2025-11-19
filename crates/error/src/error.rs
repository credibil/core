use std::error::Error as StdError;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
pub enum Error {
    /// Bad Request (400)
    /// Reserved for parsing/validation errors
    #[error("{{\"code\": 400, \"description\": \"{0}\"}}")]
    BadRequest(String),

    /// Unauthorized (401)
    /// Reserved for authorization errors
    #[error("{{\"code\": 401, \"description\": \"{0}\"}}")]
    Unauthorized(String),

    /// Not Found (404)
    /// Reserved for missing data errors
    #[error("{{\"code\": 404, \"description\": \"{0}\"}}")]
    NotFound(String),

    /// Gone (410)
    /// Reserved for stale data errors
    #[error("{{\"code\": 410, \"description\": \"{0}\"}}")]
    Gone(String),

    /// I'm a teapot (418)
    /// Reserved for all other errors
    #[error("{{\"code\": 418, \"description\": \"{0}\"}}")]
    ImATeaPot(String),

    /// Internal Server Error (500)
    /// Reserved for runtime errors
    #[error("{{\"code\": 500, \"description\": \"{0}\"}}")]
    ServerError(String),

    /// Bad Gateway (502)
    /// Reserved for upstream service errors (e.g. external sources, API calls, databases, etc.)
    #[error("{{\"code\": 502, \"description\": \"{0}\"}}")]
    BadGateway(String),

    /// Service Unavailable (503)
    /// Reserved for Application-level errors
    #[error("{{\"code\": 503, \"description\": \"{0}\"}}")]
    ServiceUnavailable(String),
}

impl Error {
    /// Returns the error code.
    #[must_use]
    pub const fn code(&self) -> u64 {
        match self {
            Self::BadRequest(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::NotFound(_) => 404,
            Self::Gone(_) => 410,
            Self::ImATeaPot(_) => 418,
            Self::ServerError(_) => 500,
            Self::BadGateway(_) => 502,
            Self::ServiceUnavailable(_) => 503,
        }
    }

    /// Returns the error description.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::BadRequest(desc)
            | Self::Unauthorized(desc)
            | Self::NotFound(desc)
            | Self::Gone(desc)
            | Self::ImATeaPot(desc)
            | Self::ServerError(desc)
            | Self::BadGateway(desc)
            | Self::ServiceUnavailable(desc) => desc.clone(),
        }
    }

    /// Performs tracing and metrics.
    pub fn trace(&self, service: &str, topic: &str) {
        match self {
            Self::ServiceUnavailable(description) => {
                error!(
                    monotonic_counter.processing_errors = 1,
                    service = %service,
                    topic = %topic,
                    description
                );
            }
            Self::BadGateway(description) => {
                error!(monotonic_counter.external_errors = 1, service = %service, topic = %topic, description);
            }
            Self::ServerError(description) => {
                error!(monotonic_counter.runtime_errors = 1, service = %service, description);
            }
            Self::BadRequest(description) => {
                warn!(
                    monotonic_counter.parsing_errors = 1,
                    service = %service,
                    topic = %topic,
                    description
                );
            }
            Self::Unauthorized(description) | Self::NotFound(description) => {
                info!(description);
            }
            Self::Gone(description) => {
                info!(monotonic_counter.stale_data = 1, service = %service, topic = %topic, description);
            }
            Self::ImATeaPot(description) => {
                info!(monotonic_counter.other_errors = 1, service = %service, description);
            }
        }
    }

    pub fn from_string(raw: String) -> Self {
        let obj: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return Self::ImATeaPot(raw),
        };

        let code = obj.get("code").and_then(Value::as_u64).unwrap_or(500);
        let description =
            obj.get("description").and_then(Value::as_str).unwrap_or(&raw).to_string();

        match code {
            400 => Self::BadRequest(description),
            401 => Self::Unauthorized(description),
            404 => Self::NotFound(description),
            410 => Self::Gone(description),
            502 => Self::BadGateway(description),
            503 => Self::ServiceUnavailable(description),
            500 => Self::ServerError(description),
            _ => Self::ImATeaPot(description),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::from_string(err.root_cause().to_string())
    }
}

impl From<Box<dyn StdError>> for Error {
    fn from(err: Box<dyn StdError>) -> Self {
        Self::ServerError(err.to_string())
    }
}
