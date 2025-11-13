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
    provider: P,
}

impl<P: Provider> Client<P> {
    /// Create a new `Client`.
    #[must_use]
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P: Provider> Client<P> {
    /// Create a new [`Router`] with no headers.
    #[must_use]
    pub const fn request<B: Body, U: Body, E>(
        &'_ self, body: B,
    ) -> Router<'_, P, NoOwner, NoHeaders, B, U, E> {
        Router::new(self, body)
    }
}

/// A type-safe request router that uses the type system to ensure required
/// fields are set before execution.
#[derive(Debug)]
pub struct Router<'a, P, O, H, B, U, E>
where
    P: Provider,
    H: Headers,
    B: Body,
{
    client: &'a Client<P>,
    owner: O,
    request: Request<B, H>,
    _phantom: PhantomData<fn() -> (U, E)>,
}

/// The router has no owner set.
#[doc(hidden)]
pub struct NoOwner;
/// The router has an owner set.
#[doc(hidden)]
pub struct OwnerSet<'a>(&'a str);

impl<'a, P, B, U, E> Router<'a, P, NoOwner, NoHeaders, B, U, E>
where
    P: Provider,
    B: Body,
    U: Body,
{
    /// Create a new `Router` instance.
    #[must_use]
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

// No owner set.
impl<'a, P, H, B, U, E> Router<'a, P, NoOwner, H, B, U, E>
where
    P: Provider,
    H: Headers,
    B: Body,
    U: Body,
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

/// [`NoHeaders`] headers set.
impl<'a, P, O, B, U, E> Router<'a, P, O, NoHeaders, B, U, E>
where
    P: Provider,
    B: Body,
    U: Body,
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
    U: Body + 'a,
    E: Send,
    Request<B, H>: Handler<U, P, Error = E>,
{
    /// Handle the request by routing it to the appropriate handler.
    ///
    /// # Constraints
    ///
    /// This method requires that `Request<B, H>` implements `Handler<U, P, Error = E>`.
    /// If you see an error about missing trait implementations, ensure your request
    /// type has the appropriate handler implementation.
    ///
    /// # Errors
    ///
    /// Returns the error from the underlying handler on failure.
    #[inline]
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
    U: Body + 'a,
    E: Send + 'a,
    Request<B, H>: Handler<U, P, Error = E>,
{
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;
    type Output = Result<Response<U>, E>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.handle())
    }
}

pub trait Provider: Send + Sync {}

impl<T> Provider for T where T: Send + Sync {}

/// Request handler.
///
/// The primary role of this trait is to provide a common interface for
/// requests so they can be handled by [`handle`] method.
pub trait Handler<B, P>
where
    P: Provider,
    B: Body,
{
    /// The error type returned by the handler.
    type Error;

    /// Routes the message to the concrete handler used to process the message.
    fn handle(
        self, owner: &str, provider: &P,
    ) -> impl Future<Output = Result<Response<B>, Self::Error>> + Send;
}

/// The `Headers` trait is used to restrict the types able to implement
/// request headers.
pub trait Headers: Clone + Debug + Send + Sync {}

/// Implement empty headers for use by handlers that do not require headers.
#[derive(Clone, Debug)]
pub struct NoHeaders;
impl Headers for NoHeaders {}

/// The `Body` trait is used to restrict the types able to implement
/// request body. It is implemented by all `xxxRequest` types.
pub trait Body: Clone + Debug + Send + Sync {}

impl<T> Body for T where T: Clone + Debug + Send + Sync {}

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
pub struct Response<B, H = NoHeaders>
where
    H: Headers,
    B: Body,
{
    /// Response HTTP status code.
    pub status: StatusCode,

    /// Response HTTP headers, if any.
    pub headers: Option<H>,

    /// The endpoint-specific response.
    pub body: B,
}

impl<T: Body> From<T> for Response<T> {
    fn from(body: T) -> Self {
        Self {
            status: StatusCode::OK,
            headers: None,
            body,
        }
    }
}

impl<B: Body, H: Headers> Response<B, H> {
    /// Create a success response with a specific status code.
    #[must_use]
    pub const fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Add headers to the response.
    #[must_use]
    pub fn with_headers(mut self, headers: H) -> Self {
        self.headers = Some(headers);
        self
    }
}

impl<T: Body> Deref for Response<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}
