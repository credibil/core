//! # API
//!
//! The api module provides the entry point to the public API. Requests are routed
//! to the appropriate handler for processing, returning a response that can
//! be serialized to a JSON object or directly to HTTP.

mod api;
mod http;

pub use api::*;
pub use http::*;
