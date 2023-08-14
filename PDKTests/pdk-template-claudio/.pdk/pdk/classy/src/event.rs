// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{
    convert::Infallible,
    marker::PhantomData,
    rc::Rc,
    task::{Poll, Waker},
};

use futures::Stream;
use std::future::Future;

mod private {
    pub trait Sealed {}
}

use private::Sealed;

use crate::http_constants::{
    DEFAULT_PATH, HEADER_AUTHORITY, HEADER_METHOD, HEADER_PATH, HEADER_SCHEME, HEADER_STATUS,
};
use crate::{
    extract::FromContext,
    host::Host,
    reactor::http::{ExchangePhase, HttpReactor, WakerId},
    types::HttpCid,
};

pub trait Event: Sealed {
    fn kind() -> EventKind;
}

pub trait After<S: Event>: Event {}
pub trait Before<S: Event>: Event {}

impl<A, B> Before<B> for A
where
    B: After<A>,
    A: Event,
{
}

pub trait Body: Event {}

/// Alias name for CreateContext event
pub enum Start {}
pub enum RequestHeaders {}
pub enum RequestBody {}
pub enum RequestTrailers {}
pub enum ResponseHeaders {}
pub enum ResponseBody {}
pub enum ResponseTrailers {}
pub enum ExchangeComplete {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EventKind {
    Start,
    RequestHeaders,
    RequestBody,
    RequestTrailers,
    ResponseHeaders,
    ResponseBody,
    ResponseTrailers,
    ExchangeComplete,
}

impl PartialOrd for EventKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventKind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as i32).cmp(&(*other as i32))
    }
}

impl Sealed for Start {}
impl Event for Start {
    fn kind() -> EventKind {
        EventKind::Start
    }
}

impl Sealed for RequestHeaders {}
impl Event for RequestHeaders {
    fn kind() -> EventKind {
        EventKind::RequestHeaders
    }
}
impl After<Start> for RequestHeaders {}

impl Sealed for RequestBody {}
impl Event for RequestBody {
    fn kind() -> EventKind {
        EventKind::RequestBody
    }
}
impl After<Start> for RequestBody {}
impl After<RequestHeaders> for RequestBody {}
impl Body for RequestBody {}

impl Sealed for RequestTrailers {}
impl Event for RequestTrailers {
    fn kind() -> EventKind {
        EventKind::RequestTrailers
    }
}
impl After<Start> for RequestTrailers {}
impl After<RequestHeaders> for RequestTrailers {}
impl After<RequestBody> for RequestTrailers {}

impl Sealed for ResponseHeaders {}
impl Event for ResponseHeaders {
    fn kind() -> EventKind {
        EventKind::ResponseHeaders
    }
}
impl After<Start> for ResponseHeaders {}
impl After<RequestHeaders> for ResponseHeaders {}
impl After<RequestBody> for ResponseHeaders {}
impl After<RequestTrailers> for ResponseHeaders {}

impl Sealed for ResponseBody {}
impl Event for ResponseBody {
    fn kind() -> EventKind {
        EventKind::ResponseBody
    }
}

impl After<Start> for ResponseBody {}
impl After<RequestHeaders> for ResponseBody {}
impl After<RequestBody> for ResponseBody {}
impl After<RequestTrailers> for ResponseBody {}
impl After<ResponseHeaders> for ResponseBody {}
impl Body for ResponseBody {}

impl Sealed for ResponseTrailers {}
impl Event for ResponseTrailers {
    fn kind() -> EventKind {
        EventKind::ResponseTrailers
    }
}

impl After<Start> for ResponseTrailers {}
impl After<RequestHeaders> for ResponseTrailers {}
impl After<RequestBody> for ResponseTrailers {}
impl After<RequestTrailers> for ResponseTrailers {}
impl After<ResponseHeaders> for ResponseTrailers {}
impl After<ResponseBody> for ResponseTrailers {}

impl Sealed for ExchangeComplete {}
impl Event for ExchangeComplete {
    fn kind() -> EventKind {
        EventKind::ExchangeComplete
    }
}
impl After<Start> for ExchangeComplete {}

pub struct Exchange<S: Event> {
    reactor: Rc<HttpReactor>,
    host: Rc<dyn Host>,
    _phantom: PhantomData<S>,
}

pub struct EventData<'a, S: Event> {
    exchange: &'a Exchange<S>,
}

impl<'a, S: Event> EventData<'a, S> {
    pub(crate) fn new(exchange: &'a Exchange<S>) -> Self {
        Self { exchange }
    }
}

impl<S> FromContext<EventData<'_, S>> for Rc<dyn Host>
where
    S: Event,
{
    type Error = Infallible;

    fn from_context(event: &EventData<'_, S>) -> Result<Self, Self::Error> {
        Ok(event.exchange.host.clone())
    }
}

impl<S> FromContext<EventData<'_, S>> for Rc<HttpReactor>
where
    S: Event,
{
    type Error = Infallible;

    fn from_context(event: &EventData<'_, S>) -> Result<Self, Self::Error> {
        Ok(event.exchange.reactor.clone())
    }
}

pub trait HeadersAccessor {
    fn header(&self, name: &str) -> Option<String>;

    fn headers(&self) -> Vec<(String, String)>;

    fn add_header(&self, name: &str, value: &str);

    fn set_header(&self, name: &str, value: &str);

    fn set_headers(&self, headers: Vec<(&str, &str)>);

    fn remove_header(&self, name: &str);
}

impl<'a> EventData<'a, RequestHeaders> {
    pub fn method(&self) -> String {
        self.header(HEADER_METHOD).unwrap_or_default()
    }

    pub fn scheme(&self) -> String {
        self.header(HEADER_SCHEME).unwrap_or_default()
    }

    pub fn authority(&self) -> String {
        self.header(HEADER_AUTHORITY).unwrap_or_default()
    }

    pub fn path(&self) -> String {
        self.header(HEADER_PATH)
            .unwrap_or_else(|| DEFAULT_PATH.to_string())
    }
}

impl<'a> EventData<'a, ResponseHeaders> {
    pub fn status_code(&self) -> u32 {
        self.header(HEADER_STATUS)
            .and_then(|status| status.parse::<u32>().ok())
            .unwrap_or_default()
    }
}

impl<'a> HeadersAccessor for EventData<'a, RequestHeaders> {
    fn header(&self, name: &str) -> Option<String> {
        self.exchange.host.get_http_request_header(name)
    }

    fn headers(&self) -> Vec<(String, String)> {
        self.exchange.host.get_http_request_headers()
    }

    fn add_header(&self, name: &str, value: &str) {
        self.exchange.host.add_http_request_header(name, value);
    }

    fn set_header(&self, name: &str, value: &str) {
        self.exchange
            .host
            .set_http_request_header(name, Some(value));
    }

    fn set_headers(&self, headers: Vec<(&str, &str)>) {
        self.exchange.host.set_http_request_headers(headers);
    }

    fn remove_header(&self, name: &str) {
        self.exchange.host.set_http_request_header(name, None);
    }
}

impl<'a> EventData<'a, RequestTrailers> {
    pub fn header(&self, name: &str) -> Option<String> {
        self.exchange.host.get_http_request_trailer(name)
    }

    pub fn headers(&self) -> Vec<(String, String)> {
        self.exchange.host.get_http_request_trailers()
    }
}

impl<'a> HeadersAccessor for EventData<'a, ResponseHeaders> {
    fn header(&self, name: &str) -> Option<String> {
        self.exchange.host.get_http_response_header(name)
    }

    fn headers(&self) -> Vec<(String, String)> {
        self.exchange.host.get_http_response_headers()
    }

    fn add_header(&self, name: &str, value: &str) {
        self.exchange.host.add_http_response_header(name, value);
    }

    fn set_header(&self, name: &str, value: &str) {
        self.exchange
            .host
            .set_http_response_header(name, Some(value));
    }

    fn set_headers(&self, headers: Vec<(&str, &str)>) {
        self.exchange.host.set_http_response_headers(headers);
    }

    fn remove_header(&self, name: &str) {
        self.exchange.host.set_http_response_header(name, None);
    }
}

impl<S: Event> Exchange<S> {
    pub(crate) fn new(reactor: Rc<HttpReactor>, host: Rc<dyn Host>) -> Self {
        Self {
            reactor,
            host,
            _phantom: PhantomData::default(),
        }
    }

    pub(crate) fn context_id(&self) -> HttpCid {
        self.reactor.context_id()
    }

    pub fn event_data(&self) -> Option<EventData<S>> {
        (self.reactor.current_event() == S::kind()).then(|| EventData::new(self))
    }

    pub(crate) fn wait_for_event<E>(self) -> ExchangeFuture<E>
    where
        E: Event,
        S: Before<E>,
    {
        ExchangeFuture::new(self.context_id(), self.reactor, self.host)
    }

    pub async fn wait_for_request_headers(self) -> Exchange<RequestHeaders>
    where
        S: Before<RequestHeaders>,
    {
        self.wait_for_event().await
    }

    pub(crate) async fn _wait_for_request_body(self) -> Exchange<RequestBody>
    where
        S: Before<RequestBody>,
    {
        self.wait_for_event().await
    }

    pub(crate) async fn _wait_for_request_trailers(self) -> Exchange<RequestTrailers>
    where
        S: Before<RequestTrailers>,
    {
        self.wait_for_event().await
    }

    pub async fn wait_for_response_headers(self) -> Exchange<ResponseHeaders>
    where
        S: Before<ResponseHeaders>,
    {
        self.wait_for_event().await
    }

    pub(crate) async fn _wait_for_response_body(self) -> Exchange<ResponseBody>
    where
        S: Before<ResponseBody>,
    {
        self.wait_for_event().await
    }

    pub(crate) async fn _wait_for_response_trailers(self) -> Exchange<ResponseTrailers>
    where
        S: Before<ResponseTrailers>,
    {
        self.wait_for_event().await
    }

    pub(crate) async fn _wait_for_exchange_complete(self) -> Exchange<ExchangeComplete>
    where
        S: Before<ExchangeComplete>,
    {
        self.wait_for_event().await
    }
}

impl<S> Exchange<S>
where
    S: After<Start> + Before<ResponseHeaders>,
{
    pub fn send_response(self, status_code: u32, headers: Vec<(&str, &str)>, body: Option<&[u8]>) {
        self.reactor.set_paused(true);
        self.reactor.cancel_request();
        self.host.send_http_response(status_code, headers, body);
    }
}

pub struct ExchangeFuture<S: Event> {
    _context_id: HttpCid,
    reactor: Rc<HttpReactor>,
    host: Rc<dyn Host>,
    id_and_waker: Option<(WakerId, Waker)>,
    _phantom: PhantomData<S>,
}

impl<S: Event> ExchangeFuture<S> {
    fn new(context_id: HttpCid, reactor: Rc<HttpReactor>, host: Rc<dyn Host>) -> Self {
        Self {
            _context_id: context_id,
            reactor,
            host,
            id_and_waker: None,
            _phantom: PhantomData::default(),
        }
    }
}

impl<S: Event> Unpin for ExchangeFuture<S> {}

impl<S: Event> Future for ExchangeFuture<S> {
    type Output = Exchange<S>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if self.reactor.paused() {
            self.reactor.set_paused(false);
            match self.reactor.phase() {
                ExchangePhase::Request => self.host.resume_http_request(),
                ExchangePhase::Response => self.host.resume_http_response(),
            }
        }

        if self.reactor.current_event() >= S::kind() {
            if let Some((id, _)) = self.id_and_waker.take() {
                // Deregister the waker from the reactor.
                self.reactor.remove_waker(S::kind(), id);
            }
            Poll::Ready(Exchange::new(
                Rc::clone(&self.reactor),
                Rc::clone(&self.host),
            ))
        } else {
            match &self.id_and_waker {
                None => {
                    // Register the waker in the reactor.
                    let id = self.reactor.insert_waker(S::kind(), cx.waker().clone());
                    self.id_and_waker = Some((id, cx.waker().clone()));
                }
                Some((id, w)) if !w.will_wake(cx.waker()) => {
                    // Deregister the waker from the reactor to remove the old waker.
                    self.reactor.remove_waker(S::kind(), *id);

                    // Register the waker in the reactor with the new waker.
                    let id = self.reactor.insert_waker(S::kind(), cx.waker().clone());
                    self.id_and_waker = Some((id, cx.waker().clone()));
                }
                Some(_) => {}
            }
            Poll::Pending
        }
    }
}

pub struct BodyChunk {
    size: usize,
}

impl BodyChunk {
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn bytes(self) -> Vec<u8> {
        todo!()
    }
}

impl<'a, S: Body> EventData<'a, S> {
    pub fn chunks(&self) -> BodyChunkStream<'a, S> {
        BodyChunkStream::new(self.exchange)
    }

    pub fn bytes(&self) -> BodyBytesStream<'a, S> {
        BodyBytesStream::new(self.exchange)
    }
}

pub struct BodyChunkStream<'a, S: Body> {
    _exchange: &'a Exchange<S>,
}

impl<'a, S: Body> BodyChunkStream<'a, S> {
    fn new(exchange: &'a Exchange<S>) -> Self {
        Self {
            _exchange: exchange,
        }
    }
}

impl<S: Body> Stream for BodyChunkStream<'_, S> {
    type Item = BodyChunk;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

pub struct BodyBytesStream<'a, S: Body> {
    _exchange: &'a Exchange<S>,
}

impl<'a, S: Body> BodyBytesStream<'a, S> {
    fn new(exchange: &'a Exchange<S>) -> Self {
        Self {
            _exchange: exchange,
        }
    }
}

impl<'a, S: Body> Stream for BodyBytesStream<'a, S> {
    type Item = Vec<u8>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}
