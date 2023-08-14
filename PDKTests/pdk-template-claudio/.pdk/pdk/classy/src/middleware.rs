// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::{
    event::{Event, EventData, RequestHeaders, ResponseHeaders},
    BoxError,
};

pub type EventHandlerResult = Result<(), BoxError>;

pub trait EventHandler<S>
where
    S: Event,
{
    fn call(&mut self, event: &EventData<S>) -> EventHandlerResult;
}

impl<F, S> EventHandler<S> for F
where
    F: for<'a> FnMut(&EventData<'a, S>) -> EventHandlerResult,
    S: Event,
{
    fn call(&mut self, event: &EventData<S>) -> EventHandlerResult {
        self(event)
    }
}

pub trait EventHandlerPush<S>
where
    S: Event,
{
    fn push<H>(&mut self, handler: H)
    where
        H: EventHandler<S> + 'static;
}

pub trait EventHandlerDispatch<S>
where
    S: Event + Sized,
{
    fn dispatch(&mut self, event: &EventData<S>) -> Result<(), BoxError>;
}

#[derive(Default)]
pub struct EventHandlerStack {
    request_headers_handlers: Vec<Box<dyn EventHandler<RequestHeaders>>>,
    response_headers_handlers: Vec<Box<dyn EventHandler<ResponseHeaders>>>,
}

impl EventHandlerPush<RequestHeaders> for EventHandlerStack {
    fn push<H>(&mut self, handler: H)
    where
        H: EventHandler<RequestHeaders> + 'static,
    {
        self.request_headers_handlers.push(Box::new(handler))
    }
}

impl EventHandlerPush<ResponseHeaders> for EventHandlerStack {
    fn push<H>(&mut self, handler: H)
    where
        H: EventHandler<ResponseHeaders> + 'static,
    {
        self.response_headers_handlers.push(Box::new(handler))
    }
}

impl EventHandlerDispatch<RequestHeaders> for EventHandlerStack {
    fn dispatch(&mut self, event: &EventData<RequestHeaders>) -> Result<(), BoxError> {
        for h in &mut self.request_headers_handlers {
            h.call(event)?;
        }
        Ok(())
    }
}

impl EventHandlerDispatch<ResponseHeaders> for EventHandlerStack {
    fn dispatch(&mut self, event: &EventData<ResponseHeaders>) -> Result<(), BoxError> {
        for h in &mut self.response_headers_handlers {
            h.call(event)?;
        }
        Ok(())
    }
}
