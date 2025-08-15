//! # Telemtry
//!
//! Telemetry is a module that provides functionality for collecting and
//! reporting OpenTelemetry-based metrics.

mod init;
mod tracing;

pub use tracing::*;
pub use init::Otel;