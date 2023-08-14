// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    rc::Rc,
    task::{Poll, Waker},
    time::Duration,
};

use proxy_wasm::types::{Bytes, Status};

use crate::http_constants::{
    DEFAULT_PATH, DEFAULT_TIMEOUT, HEADER_AUTHORITY, HEADER_METHOD, HEADER_PATH, HEADER_STATUS,
    METHOD_DELETE, METHOD_GET, METHOD_OPTIONS, METHOD_POST, METHOD_PUT,
};
use crate::{
    event::EventKind,
    extract::{Extract, FromContext},
    host::Host,
    reactor::root::{BoxedExtractor, RootReactor},
    types::{Cid, RequestId},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpCallResponse {
    pub request_id: RequestId,
    pub num_headers: usize,
    pub body_size: usize,
    pub num_trailers: usize,
}

pub struct HttpClient {
    reactor: Rc<RootReactor>,
    host: Rc<dyn Host>,
}

#[derive(thiserror::Error, Debug)]
pub enum HttpClientRequestError {
    #[error("Proxy status problem: {0:?}")]
    Status(Status),
}

#[derive(thiserror::Error, Debug)]
pub enum HttpClientResponseError {
    #[error("Request awaited on create context event")]
    AwaitedOnCreateContext,
}

impl HttpClient {
    pub(crate) fn new(reactor: Rc<RootReactor>, host: Rc<dyn Host>) -> Self {
        Self { reactor, host }
    }

    pub fn request<'a>(
        &'a self,
        upstream: &'a str,
        authority: &'a str,
    ) -> RequestBuilder<'a, EmptyResponseExtractor> {
        RequestBuilder::new(self, upstream, authority, EmptyResponseExtractor)
    }
}

impl<C> FromContext<C> for HttpClient
where
    Rc<dyn Host>: FromContext<C, Error = Infallible>,
    Rc<RootReactor>: FromContext<C, Error = Infallible>,
{
    type Error = Infallible;

    fn from_context(context: &C) -> Result<Self, Self::Error> {
        let reactor = context.extract()?;
        let host = context.extract()?;
        Ok(Self::new(reactor, host))
    }
}

pub struct Request<T> {
    reactor: Rc<RootReactor>,
    request_id: RequestId,
    cid_and_waker: Option<(Cid, Waker)>,
    _response_type: PhantomData<T>,
}

impl<T> Request<T> {
    pub fn id(&self) -> RequestId {
        self.request_id
    }
}

pub trait ResponseBuffers {
    fn status_code(&self) -> u32;
    fn header(&self, name: &str) -> Option<String>;
    fn headers(&self) -> Vec<(String, String)>;
    fn body(&self, start: usize, max_size: usize) -> Option<Bytes>;
    fn trailers(&self) -> Vec<(String, String)>;
}

impl ResponseBuffers for Rc<dyn Host> {
    fn status_code(&self) -> u32 {
        self.header(HEADER_STATUS)
            .and_then(|status| status.parse::<u32>().ok())
            .unwrap_or_default()
    }

    fn header(&self, name: &str) -> Option<String> {
        self.get_http_call_response_header(name)
    }

    fn headers(&self) -> Vec<(String, String)> {
        self.get_http_call_response_headers()
    }

    fn body(&self, start: usize, max_size: usize) -> Option<Bytes> {
        self.get_http_call_response_body(start, max_size)
    }

    fn trailers(&self) -> Vec<(String, String)> {
        self.get_http_call_response_trailers()
    }
}

pub trait ResponseExtractor {
    type Output;

    fn extract(self, event: &HttpCallResponse, buffers: &dyn ResponseBuffers) -> Self::Output;
}

pub struct FnResponseExtractor<F> {
    function: F,
}

impl<F, T> ResponseExtractor for FnResponseExtractor<F>
where
    F: FnOnce(&HttpCallResponse, &dyn ResponseBuffers) -> T,
{
    type Output = T;

    fn extract(self, event: &HttpCallResponse, buffers: &dyn ResponseBuffers) -> Self::Output {
        (self.function)(event, buffers)
    }
}

impl<F, T> FnResponseExtractor<F>
where
    F: FnOnce(&HttpCallResponse, &dyn ResponseBuffers) -> T,
{
    pub fn from_fn(function: F) -> FnResponseExtractor<F>
    where
        F: FnOnce(&HttpCallResponse, &dyn ResponseBuffers) -> T,
    {
        FnResponseExtractor { function }
    }
}

pub struct RequestBuilder<'a, E> {
    client: &'a HttpClient,
    extractor: E,
    upstream: &'a str,
    authority: &'a str,
    path: Option<&'a str>,
    headers: Option<Vec<(&'a str, &'a str)>>,
    body: Option<&'a [u8]>,
    trailers: Option<Vec<(&'a str, &'a str)>>,
    timeout: Option<Duration>,
}

impl<'a, E> RequestBuilder<'a, E>
where
    E: ResponseExtractor + 'static,
    E::Output: 'static,
{
    fn new(client: &'a HttpClient, upstream: &'a str, authority: &'a str, extractor: E) -> Self {
        Self {
            client,
            extractor,
            upstream,
            authority,
            path: None,
            headers: None,
            body: None,
            trailers: None,
            timeout: None,
        }
    }

    pub fn extractor<T>(self, extractor: T) -> RequestBuilder<'a, T>
    where
        T: ResponseExtractor,
    {
        RequestBuilder {
            client: self.client,
            extractor,
            upstream: self.upstream,
            authority: self.authority,
            path: self.path,
            headers: self.headers,
            body: self.body,
            trailers: self.trailers,
            timeout: self.timeout,
        }
    }

    pub fn extract_with<F, T>(self, function: F) -> RequestBuilder<'a, FnResponseExtractor<F>>
    where
        F: FnOnce(&HttpCallResponse, &dyn ResponseBuffers) -> T,
    {
        self.extractor(FnResponseExtractor::from_fn(function))
    }

    pub fn path(mut self, path: &'a str) -> Self {
        self.path = Some(path);
        self
    }

    pub fn headers(mut self, headers: Vec<(&'a str, &'a str)>) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn body(mut self, body: &'a [u8]) -> Self {
        self.body = Some(body);
        self
    }

    pub fn trailers(mut self, trailers: Vec<(&'a str, &'a str)>) -> Self {
        self.trailers = Some(trailers);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn post(self) -> Result<Request<E::Output>, HttpClientRequestError> {
        self.send(METHOD_POST)
    }
    pub fn put(self) -> Result<Request<E::Output>, HttpClientRequestError> {
        self.send(METHOD_PUT)
    }
    pub fn get(self) -> Result<Request<E::Output>, HttpClientRequestError> {
        self.send(METHOD_GET)
    }
    pub fn options(self) -> Result<Request<E::Output>, HttpClientRequestError> {
        self.send(METHOD_OPTIONS)
    }
    pub fn delete(self) -> Result<Request<E::Output>, HttpClientRequestError> {
        self.send(METHOD_DELETE)
    }

    pub fn send(mut self, method: &str) -> Result<Request<E::Output>, HttpClientRequestError> {
        let mut headers = self.headers.take().unwrap_or_default();

        headers.push((HEADER_PATH, self.path.unwrap_or(DEFAULT_PATH)));
        headers.push((HEADER_AUTHORITY, self.authority));
        headers.push((HEADER_METHOD, method));

        let body = self.body.take();
        let trailers = self.trailers.take().unwrap_or_default();
        let timeout = self.timeout.take().unwrap_or(DEFAULT_TIMEOUT);

        let request_id: RequestId = self
            .client
            .host
            .dispatch_http_call(self.upstream, headers, body, trailers, timeout)
            .map_err(HttpClientRequestError::Status)?
            .into();

        let extractor = boxed_extractor(self.client.host.clone(), self.extractor);

        self.client.reactor.insert_extractor(request_id, extractor);

        Ok(Request::new(self.client.reactor.clone(), request_id))
    }
}

impl<'a, E: ResponseExtractor> ResponseExtractor for RequestBuilder<'a, E> {
    type Output = E::Output;

    fn extract(self, event: &HttpCallResponse, buffers: &dyn ResponseBuffers) -> Self::Output {
        self.extractor.extract(event, buffers)
    }
}

fn boxed_extractor<E>(buffers: Rc<dyn Host>, extractor: E) -> BoxedExtractor
where
    E: ResponseExtractor + 'static,
    E::Output: 'static,
{
    Box::new(move |event| Box::new(extractor.extract(event, &buffers)))
}

pub struct EmptyResponseExtractor;

impl ResponseExtractor for EmptyResponseExtractor {
    type Output = ();

    fn extract(self, _event: &HttpCallResponse, _buffers: &dyn ResponseBuffers) -> Self::Output {}
}

impl<T> Request<T> {
    fn new(reactor: Rc<RootReactor>, request_id: RequestId) -> Self {
        Request {
            reactor,
            request_id,
            cid_and_waker: None,
            _response_type: PhantomData::default(),
        }
    }
}

impl<T> Drop for Request<T> {
    fn drop(&mut self) {
        let reactor = self.reactor.as_ref();

        // Ensure that all related objects were removed
        reactor.remove_extractor(self.request_id);
        reactor.remove_response(self.request_id);
        reactor.remove_client(self.request_id);
    }
}

impl<T: Unpin + 'static> Future for Request<T> {
    type Output = Result<T, HttpClientResponseError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.reactor.current_event() == Some(EventKind::Start) {
            return Poll::Ready(Err(HttpClientResponseError::AwaitedOnCreateContext));
        }

        if let Some((_event, content)) = self.reactor.remove_response(self.request_id) {
            // It should be safe to unwrap here
            let content = content.expect("response content should have been extracted");

            // It should be safe to unwrap here
            let content = content.downcast().expect("downcasting");

            Poll::Ready(Ok(*content))
        } else {
            let this = &mut *self.as_mut();
            match this.cid_and_waker.as_ref() {
                None => {
                    let cid = this.reactor.active_cid();

                    // Register the waker in the reactor.
                    this.reactor
                        .insert_client(this.request_id, cx.waker().clone());
                    this.reactor.set_paused(cid, true);
                    this.cid_and_waker = Some((cid, cx.waker().clone()));
                }
                Some((cid, waker)) if !waker.will_wake(cx.waker()) => {
                    // Deregister the waker from the reactor to remove the old waker.
                    let _ = this
                        .reactor
                        .remove_client(this.request_id)
                        // It should be safe to unwrap here
                        .expect("stored extractor");

                    // Register the waker in the reactor with the new waker.
                    this.reactor
                        .insert_client(this.request_id, cx.waker().clone());
                    this.cid_and_waker = Some((*cid, cx.waker().clone()));
                }
                Some(_) => {}
            }
            Poll::Pending
        }
    }
}
