//! # API
//!
//! The api module provides the entry point to the public API. Requests are routed
//! to the appropriate handler for processing, returning a response that can
//! be serialized to a JSON object or directly to HTTP.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use common::api::{Client, Body, Headers};
//!
//! // Create a client
//! let client = Client::new(provider);
//!
//! // Simple request without headers
//! let response = client.request(my_request).owner("alice").await?;
//!
//! // Request with headers
//! let response = client.request(my_request).owner("alice").headers(my_headers).await?;
//! ```

use std::fmt::Debug;
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;

use http::StatusCode;

/// Build an API client to execute the request.
///
/// The client is the main entry point for making API requests. It holds
/// the provider configuration and provides methods to create the request
/// router.
#[derive(Clone, Debug)]
pub struct Client<P: Provider> {
    /// The provider to use while handling of the request.
    pub provider: P,
}

impl<P: Provider> Client<P> {
    /// Create a new `Client`.
    #[must_use]
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P: Provider> Client<P> {
    /// Create a new `Request` with no headers.
    pub const fn request<B: Body, U, E>(
        &'_ self, body: B,
    ) -> Router<'_, P, NoOwner, NoHeaders, B, U, E> {
        Router::new(self, body)
    }
}

/// A type-safe request builder that uses the type system to ensure required
/// fields are set before execution.
#[derive(Debug)]
pub struct Router<'a, P, O, H, B, U, E>
where
    P: Provider,
    B: Body,
    H: Headers,
{
    client: &'a Client<P>,
    owner: O,
    request: Request<B, H>,
    _phantom: PhantomData<(U, E)>,
}

/// The request has no owner set.
#[doc(hidden)]
pub struct NoOwner;
/// The request has a owner set.
#[doc(hidden)]
pub struct OwnerSet<'a>(&'a str);

impl<'a, P, B, U, E> Router<'a, P, NoOwner, NoHeaders, B, U, E>
where
    P: Provider,
    B: Body,
{
    /// Create a new `Router` instance.
    const fn new(client: &'a Client<P>, body: B) -> Self {
        Self {
            client,
            owner: NoOwner,
            request: Request {
                body,
                headers: NoHeaders,
            },
            _phantom: PhantomData,
        }
    }
}

// No owner.
impl<'a, P, H, B, U, E> Router<'a, P, NoOwner, H, B, U, E>
where
    P: Provider,
    B: Body,
    H: Headers,
{
    /// Set the owner (tenant).
    #[must_use]
    pub fn owner<'o>(self, owner: &'o str) -> Router<'a, P, OwnerSet<'o>, H, B, U, E> {
        Router {
            client: self.client,
            owner: OwnerSet(owner),
            request: self.request,
            _phantom: PhantomData,
        }
    }
}

/// [`NoHeaders`] headers.
impl<'a, P, O, B, U, E> Router<'a, P, O, NoHeaders, B, U, E>
where
    P: Provider,
    B: Body,
{
    /// Set request headers.
    #[must_use]
    pub fn headers<H: Headers>(self, headers: H) -> Router<'a, P, O, H, B, U, E> {
        Router {
            client: self.client,
            owner: self.owner,
            request: Request {
                body: self.request.body,
                headers,
            },
            _phantom: PhantomData,
        }
    }
}

// Owner set, maybe headers set: request can be routed to it's handler.
impl<'a, P, H, B, U, E> Router<'a, P, OwnerSet<'a>, H, B, U, E>
where
    P: Provider,
    H: Headers + 'a,
    B: Body + 'a,
    U: Send + 'a,
    E: Send,
    Request<B, H>: Handler<U, P, Error = E>,
{
    /// Handle the request by routing it to the appropriate handler.
    ///
    /// # Errors
    ///
    /// Returns the error from the underlying handler on failure.
    pub async fn handle(self) -> Result<Response<U>, E> {
        self.request.handle(self.owner.0, &self.client.provider).await
    }
}

// Implement [`IntoFuture`] so that the request can be awaited directly (without
// needing to call the `handle` method).
impl<'a, P, H, B, U, E> IntoFuture for Router<'a, P, OwnerSet<'a>, H, B, U, E>
where
    P: Provider,
    H: Headers + 'a,
    B: Body + 'a,
    U: Send + 'a,
    E: Send + 'a,
    Request<B, H>: Handler<U, P, Error = E>,
{
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;
    type Output = Result<Response<U>, E>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.handle())
    }
}

/// A request to process.
#[derive(Clone, Debug)]
pub struct Request<B, H = NoHeaders>
where
    H: Headers,
    B: Body,
{
    /// Headers associated with this request.
    pub headers: H,

    /// The request to process.
    pub body: B,
}

impl<B: Body> From<B> for Request<B> {
    fn from(body: B) -> Self {
        Self {
            body,
            headers: NoHeaders,
        }
    }
}

/// Top-level response data structure common to all handler.
#[derive(Clone, Debug)]
pub struct Response<O, H = NoHeaders>
where
    H: Headers,
{
    /// Response HTTP status code.
    pub status: StatusCode,

    /// Response HTTP headers, if any.
    pub headers: Option<H>,

    /// The endpoint-specific response.
    pub body: O,
}

impl<T> From<T> for Response<T> {
    fn from(body: T) -> Self {
        Self {
            status: StatusCode::OK,
            headers: None,
            body,
        }
    }
}

impl<T> Deref for Response<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

/// Request handler.
///
/// The primary role of this trait is to provide a common interface for
/// requests so they can be handled by [`handle`] method.
pub trait Handler<U, P> {
    /// The error type returned by the handler.
    type Error;

    /// Routes the message to the concrete handler used to process the message.
    fn handle(
        self, owner: &str, provider: &P,
    ) -> impl Future<Output = Result<Response<U>, Self::Error>> + Send;
}

/// The `Body` trait is used to restrict the types able to implement
/// request body. It is implemented by all `xxxRequest` types.
pub trait Body: Clone + Debug + Send + Sync {}

/// The `Headers` trait is used to restrict the types able to implement
/// request headers.
pub trait Headers: Clone + Debug + Send + Sync {}

/// Implement empty headers for use by handlers that do not require headers.
#[derive(Clone, Debug)]
pub struct NoHeaders;
impl Headers for NoHeaders {}

pub trait Provider: Send + Sync {}

impl<T> Provider for T where T: Send + Sync {}
