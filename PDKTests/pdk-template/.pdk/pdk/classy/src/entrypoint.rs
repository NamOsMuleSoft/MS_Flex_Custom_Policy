// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::rc::Rc;

use proxy_wasm::traits::RootContext;

use crate::bootstrap::Launcher;
use crate::context::root::AsyncRootContext;
use crate::event::{After, Exchange, Start};
use crate::extract::context::{ConfigureContext, FilterContext};
use crate::extract::FromContext;
use crate::handler::Handler;
use crate::middleware::EventHandlerStack;
use crate::types::RootCid;

pub trait Entrypoint<S, T> {
    fn create_root_context(
        self,
        event_handlers: EventHandlerStack,
        context_id: u32,
    ) -> Box<dyn RootContext>;
}

impl<H, T, E> Entrypoint<Exchange<E>, T> for H
where
    H: Handler<Exchange<E>, T, Result = ()> + Clone + 'static,
    T: FromContext<FilterContext>,
    E: After<Start>,
{
    fn create_root_context(
        self,
        event_handlers: EventHandlerStack,
        context_id: u32,
    ) -> Box<dyn RootContext> {
        let entrypoint = move |launcher: Launcher| launcher.launch(self.clone());
        entrypoint.create_root_context(event_handlers, context_id)
    }
}

impl<H, T> Entrypoint<Launcher, T> for H
where
    H: Handler<Launcher, T> + 'static,
    T: FromContext<ConfigureContext> + 'static,
{
    fn create_root_context(
        self,
        event_handlers: EventHandlerStack,
        context_id: u32,
    ) -> Box<dyn RootContext> {
        Box::new(AsyncRootContext::new(
            RootCid::from(context_id),
            Rc::new(crate::host::DefaultHost),
            event_handlers,
            self,
        ))
    }
}
