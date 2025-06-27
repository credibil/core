//! # API
//!
//! The api module provides the entry point to the public API. Requests are routed
//! to the appropriate handler for processing, returning a response that can
//! be serialized to a JSON object or directly to HTTP.

use std::fmt::Debug;
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::ops::Deref;

use http::StatusCode;
use tracing::instrument;

/// Build an API `Client` to execute the request.
#[derive(Clone, Debug)]
pub struct Client<P: Send + Sync> {
    /// The provider to use while handling of the request.
    pub provider: P,
}

impl<P: Send + Sync> Client<P> {
    /// Create a new `Client`.
    #[must_use]
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P: Send + Sync> Client<P> {
    /// Create a new `Request` with no headers.
    pub const fn request<B: Body, U, E>(
        &'_ self, body: B,
    ) -> RequestBuilder<'_, P, NoOwner, NoHeader, B, U, E> {
        RequestBuilder::new(self, body)
    }
}

/// Request builder.
#[derive(Debug)]
pub struct RequestBuilder<'a, P, O, H, B, U, E>
where
    P: Send + Sync,
    B: Body,
{
    client: &'a Client<P>,
    owner: O,
    headers: H,
    body: B,

    _phantom: PhantomData<(U, E)>,
}

/// The request has no headers.
#[doc(hidden)]
pub struct NoHeader;
/// The request has headers.
#[doc(hidden)]
pub struct HeaderSet<H: Headers>(H);

/// The request has no owner set.
#[doc(hidden)]
pub struct NoOwner;
/// The request has a owner set.
#[doc(hidden)]
pub struct OwnerSet<'a>(&'a str);

impl<'a, P, B, U, E> RequestBuilder<'a, P, NoOwner, NoHeader, B, U, E>
where
    P: Send + Sync,
    B: Body,
{
    /// Create a new `Request` instance.
    pub const fn new(client: &'a Client<P>, body: B) -> Self {
        Self {
            client,
            owner: NoOwner,
            headers: NoHeader,
            body,
            _phantom: PhantomData,
        }
    }
}

impl<'a, P, H, B, U, E> RequestBuilder<'a, P, NoOwner, H, B, U, E>
where
    P: Send + Sync,
    B: Body,
{
    /// Set the headers for the request.
    #[must_use]
    pub fn owner<'o>(self, owner: &'o str) -> RequestBuilder<'a, P, OwnerSet<'o>, H, B, U, E> {
        RequestBuilder {
            client: self.client,
            headers: self.headers,
            owner: OwnerSet(owner),
            body: self.body,
            _phantom: PhantomData,
        }
    }
}

impl<'a, P, O, B, U, E> RequestBuilder<'a, P, O, NoHeader, B, U, E>
where
    P: Send + Sync,
    B: Body,
{
    /// Set the headers for the request.
    #[must_use]
    pub fn headers<H: Headers>(
        self, headers: H,
    ) -> RequestBuilder<'a, P, O, HeaderSet<H>, B, U, E> {
        RequestBuilder {
            client: self.client,
            owner: self.owner,
            headers: HeaderSet(headers),
            body: self.body,
            _phantom: PhantomData,
        }
    }
}

impl<P, B, U, E> RequestBuilder<'_, P, OwnerSet<'_>, NoHeader, B, U, E>
where
    P: Send + Sync,
    B: Body,
{
    /// Process the request and return a response.
    ///
    /// # Errors
    ///
    /// Will fail if request cannot be processed.
    #[instrument(level = "debug", skip(self))]
    pub async fn execute(self) -> Result<Response<U>, E>
    where
        B: Body,
        Request<B, Empty>: Handler<U, P, Error = E> + From<B>,
    {
        let request: Request<B, Empty> = self.body.into();
        Ok(request.handle(self.owner.0, &self.client.provider).await?.into())
    }
}

impl<P, H, B, U, E> RequestBuilder<'_, P, OwnerSet<'_>, HeaderSet<H>, B, U, E>
where
    P: Send + Sync,
    B: Body,
    H: Headers,
{
    /// Process the request and return a response.
    ///
    /// # Errors
    ///
    /// Will fail if request cannot be processed.
    pub async fn execute(self) -> Result<Response<U>, E>
    where
        B: Body,
        Request<B, H>: Handler<U, P, Error = E>,
    {
        let request = Request {
            body: self.body,
            headers: self.headers.0.clone(),
        };
        Ok(request.handle(self.owner.0, &self.client.provider).await?.into())
    }
}

impl<P, B, U, E> IntoFuture for RequestBuilder<'_, P, OwnerSet<'_>, NoHeader, B, U, E>
where
    P: Send + Sync,
    B: Body,
    U: Send,
    E: Send,
    Request<B, Empty>: Handler<U, P, Error = E> + From<B>,
{
    type Output = Result<Response<U>, E>;

    type IntoFuture = impl Future<Output = Self::Output> + Send;

    fn into_future(self) -> Self::IntoFuture {
        self.execute()
    }
}

impl<P, H, B, U, E> IntoFuture for RequestBuilder<'_, P, OwnerSet<'_>, HeaderSet<H>, B, U, E>
where
    P: Send + Sync,
    H: Headers,
    B: Body,
    U: Send,
    E: Send,
    Request<B, H>: Handler<U, P, Error = E>,
{
    type Output = Result<Response<U>, E>;

    type IntoFuture = impl Future<Output = Self::Output> + Send;

    fn into_future(self) -> Self::IntoFuture {
        self.execute()
    }
}

/// A request to process.
#[derive(Clone, Debug)]
pub struct Request<B, H = Empty>
where
    B: Body,
    H: Headers,
{
    /// The request to process.
    pub body: B,

    /// Headers associated with this request.
    pub headers: H,
}

impl<B: Body> From<B> for Request<B> {
    fn from(body: B) -> Self {
        Self { body, headers: Empty }
    }
}

/// Top-level response data structure common to all handler.
#[derive(Clone, Debug)]
pub struct Response<O, H = Empty>
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
    ) -> impl Future<Output = Result<impl Into<Response<U>>, Self::Error>> + Send;
}

/// The `Body` trait is used to restrict the types able to implement
/// request body. It is implemented by all `xxxRequest` types.
pub trait Body: Clone + Debug + Send + Sync {}

/// The `Headers` trait is used to restrict the types able to implement
/// request headers.
pub trait Headers: Clone + Debug + Send + Sync {}

/// Implement empty headers for use by handlers that do not require headers.
#[derive(Clone, Debug)]
pub struct Empty;
impl Headers for Empty {}
