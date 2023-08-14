// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::marker::PhantomData;

use proxy_wasm::traits::RootContext;

use crate::{
    entrypoint::Entrypoint,
    event::{Event, RequestHeaders, ResponseHeaders},
    middleware::{EventHandler, EventHandlerPush, EventHandlerStack},
};

#[derive(Default)]
pub struct Plugin<E = (), T = ()> {
    event_handlers: EventHandlerStack,
    entrypoint: E,
    _types: PhantomData<T>,
}

impl EventHandlerPush<RequestHeaders> for Plugin {
    fn push<H>(&mut self, handler: H)
    where
        H: EventHandler<RequestHeaders> + 'static,
    {
        self.event_handlers.push(handler)
    }
}

impl EventHandlerPush<ResponseHeaders> for Plugin {
    fn push<H>(&mut self, handler: H)
    where
        H: EventHandler<ResponseHeaders> + 'static,
    {
        self.event_handlers.push(handler)
    }
}

impl Plugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn event_handler<H, S>(mut self, handler: H) -> Self
    where
        S: Event,
        H: EventHandler<S> + 'static,
        Self: EventHandlerPush<S>,
    {
        self.push(handler);
        self
    }

    pub fn entrypoint<C, T, E>(self, entrypoint: E) -> Plugin<E, (C, T)>
    where
        E: Entrypoint<C, T>,
    {
        Plugin {
            event_handlers: self.event_handlers,
            entrypoint,
            _types: PhantomData::default(),
        }
    }
}

impl<E, C, T> Plugin<E, (C, T)>
where
    E: Entrypoint<C, T>,
{
    pub fn create_root_context(self, context_id: u32) -> Box<dyn RootContext> {
        self.entrypoint
            .create_root_context(self.event_handlers, context_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bootstrap::Launcher,
        client::HttpClient,
        event::{EventData, Exchange, RequestHeaders, ResponseHeaders},
        Configuration, Plugin,
    };

    #[test]
    fn test_configure() {
        Plugin::new()
            .entrypoint(|_: Launcher| async {})
            .create_root_context(1);
    }

    #[test]
    fn test_filter() {
        Plugin::new()
            .entrypoint(|_: Exchange<RequestHeaders>| async {})
            .create_root_context(1);
    }

    #[test]
    fn test_configure_with_dependencies() {
        Plugin::new()
            .entrypoint(|_: Launcher, _: Configuration| async {})
            .create_root_context(1);
    }

    #[test]
    fn test_filter_with_dependencies() {
        Plugin::new()
            .entrypoint(|_: Launcher, _: HttpClient| async {})
            .create_root_context(1);
    }

    #[test]
    fn test_configure_with_event_handlers() {
        Plugin::new()
            .event_handler(|_: &EventData<RequestHeaders>| Ok(()))
            .event_handler(|_: &EventData<ResponseHeaders>| Ok(()))
            .entrypoint(|_: Launcher| async {})
            .create_root_context(1);
    }
}
