//! # API
//!
//! The api module provides the entry point to the public API. Requests are routed
//! to the appropriate handler for processing, returning a response that can
//! be serialized to a JSON object or directly to HTTP.

use std::fmt::Debug;
use std::ops::Deref;

use http::StatusCode;
use tracing::instrument;

/// Build an API `Client` to execute the request.
#[derive(Clone, Debug)]
pub struct Client<P> {
    /// The owner of the client, typically a DID or URL.
    pub owner: String,

    /// The provider to use while handling of the request.
    pub provider: P,
}

impl<P> Client<P> {
    /// Create a new `Client`.
    #[must_use]
    pub fn new(owner: impl Into<String>, provider: P) -> Self {
        Self {
            owner: owner.into(),
            provider,
        }
    }
}

impl<P> Client<P> {
    /// Create a new `Request` with no headers.
    pub const fn request<B: Body>(&'_ self, body: B) -> RequestBuilder<'_, P, Unset, B> {
        RequestBuilder::new(self, body)
    }
}

/// Request builder.
#[derive(Debug)]
pub struct RequestBuilder<'a, P, H, B: Body> {
    client: &'a Client<P>,
    headers: H,
    body: B,
}

/// The request has no headers.
#[doc(hidden)]
pub struct Unset;
/// The request has headers.
#[doc(hidden)]
pub struct HeaderSet<H: Headers>(H);

impl<'a, P, B: Body> RequestBuilder<'a, P, Unset, B> {
    /// Create a new `Request` instance.
    pub const fn new(client: &'a Client<P>, body: B) -> Self {
        Self {
            client,
            headers: Unset,
            body,
        }
    }

    /// Set the headers for the request.
    #[must_use]
    pub fn headers<H: Headers>(self, headers: H) -> RequestBuilder<'a, P, HeaderSet<H>, B> {
        RequestBuilder {
            client: self.client,
            headers: HeaderSet(headers),
            body: self.body,
        }
    }
}

impl<P, B: Body> RequestBuilder<'_, P, Unset, B> {
    /// Process the request and return a response.
    ///
    /// # Errors
    ///
    /// Will fail if request cannot be processed.
    #[instrument(level = "debug", skip(self))]
    pub async fn execute<U, E>(self) -> Result<Response<U>, E>
    where
        B: Body,
        Request<B, NoHeaders>: Handler<U, P, Error = E> + From<B>,
    {
        let request: Request<B, NoHeaders> = self.body.into();
        Ok(request.handle(&self.client.owner, &self.client.provider).await?.into())
    }
}

impl<P, H: Headers, B: Body> RequestBuilder<'_, P, HeaderSet<H>, B> {
    /// Process the request and return a response.
    ///
    /// # Errors
    ///
    /// Will fail if request cannot be processed.
    pub async fn execute<U, E>(self) -> Result<Response<U>, E>
    where
        B: Body,
        Request<B, H>: Handler<U, P, Error = E>,
    {
        let request = Request {
            body: self.body,
            headers: self.headers.0.clone(),
        };
        Ok(request.handle(&self.client.owner, &self.client.provider).await?.into())
    }
}

/// A request to process.
#[derive(Clone, Debug)]
pub struct Request<B, H = NoHeaders>
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
        Self {
            body,
            headers: NoHeaders,
        }
    }
}

/// Top-level response data structure common to all handler.
#[derive(Clone, Debug)]
pub struct Response<T, H = NoHeaders>
where
    H: Headers,
{
    /// Response HTTP status code.
    pub status: StatusCode,

    /// Response HTTP headers, if any.
    pub headers: Option<H>,

    /// The endpoint-specific response.
    pub body: T,
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
        self, tenant: &str, provider: &P,
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
pub struct NoHeaders;
impl Headers for NoHeaders {}
