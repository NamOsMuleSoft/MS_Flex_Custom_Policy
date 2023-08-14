// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{cell::RefCell, error::Error, marker::PhantomData, rc::Rc};

use futures::{executor::LocalPool, task::LocalSpawnExt, FutureExt};
use proxy_wasm::{
    traits::{Context, HttpContext, RootContext},
    types::ContextType,
};

use crate::{
    bootstrap::Launcher,
    client::HttpCallResponse,
    extract::{context::ConfigureContext, FromContext},
    handler::{Handler, IntoHandlerResult},
    host::Host,
    middleware::EventHandlerStack,
    reactor::root::RootReactor,
    types::{HttpCid, RootCid},
};

use super::{error::ErrorContext, http::AsyncHttpContext};

#[derive(Clone)]
enum ConfigurationState {
    Started,
    Finished,
    Failed(Rc<dyn Error>),
}

pub(crate) struct AsyncRootContext<C, T> {
    context_id: RootCid,
    state: Rc<RefCell<ConfigurationState>>,
    host: Rc<dyn Host>,
    executor: Rc<RefCell<LocalPool>>,
    reactor: Rc<RootReactor>,
    event_handlers: Rc<RefCell<EventHandlerStack>>,
    configure: C,
    _arguments: PhantomData<T>,
}

impl<T, C> AsyncRootContext<C, T> {
    pub(crate) fn new(
        context_id: RootCid,
        host: Rc<dyn Host>,
        event_handlers: EventHandlerStack,
        configure: C,
    ) -> Self {
        Self {
            context_id,
            state: Rc::new(RefCell::new(ConfigurationState::Started)),
            host,
            executor: Rc::new(RefCell::new(LocalPool::new())),
            reactor: Rc::new(RootReactor::new(context_id)),
            event_handlers: Rc::new(RefCell::new(event_handlers)),
            configure,
            _arguments: PhantomData::default(),
        }
    }
}

impl<C, T> Context for AsyncRootContext<C, T> {
    fn on_http_call_response(
        &mut self,
        token_id: u32,
        num_headers: usize,
        body_size: usize,
        num_trailers: usize,
    ) {
        self.reactor.notify_response(HttpCallResponse {
            request_id: token_id.into(),
            num_headers,
            body_size,
            num_trailers,
        });

        self.reactor.set_active_cid(self.context_id.into());

        self.executor.borrow_mut().run_until_stalled();
    }

    fn on_done(&mut self) -> bool {
        self.reactor.set_done();
        true
    }
}

impl<C, T> RootContext for AsyncRootContext<C, T>
where
    C: Handler<Launcher, T> + 'static,
    T: FromContext<ConfigureContext> + 'static,
{
    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }

    fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
        match *self.state.borrow() {
            ConfigurationState::Failed(ref error) => {
                let error_context = ErrorContext::new(self.host.clone(), error.clone());
                Some(Box::new(error_context))
            }
            ConfigurationState::Finished => {
                log::error!("Inconsistent filter state.");
                let error: Box<dyn Error> =
                    Box::from("Incoming request after the configure function finished");
                let error_context = ErrorContext::new(self.host.clone(), error.into());
                Some(Box::new(error_context))
            }
            ConfigurationState::Started => {
                let context_id = HttpCid::from(context_id);
                let executor = self.executor.clone();
                let host = self.host.clone();
                let config_reactor = self.reactor.clone();
                let reactor = if let Some(reactor) = config_reactor.create_http_context(context_id)
                {
                    reactor
                } else {
                    log::warn!(
                        "There is no current HttpContext available. \
                            A filter function was not launched in configure function or \
                            an async operation is still in progress."
                    );
                    let error: Box<dyn Error> = Box::from("Configuration in pending state");
                    let error_context = ErrorContext::new(self.host.clone(), error.into());
                    return Some(Box::new(error_context));
                };

                config_reactor.set_active_cid(context_id.into());
                self.executor.borrow_mut().run_until_stalled();
                let filter = AsyncHttpContext::new(
                    context_id,
                    executor,
                    host,
                    config_reactor.clone(),
                    reactor,
                    self.event_handlers.clone(),
                );

                config_reactor.set_active_cid(self.context_id.into());

                Some(Box::new(filter))
            }
        }
    }

    fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
        let host = self.host.clone();
        let reactor = self.reactor.clone();
        reactor.set_active_cid(self.context_id.into());
        let context = ConfigureContext::new(host.clone(), reactor.clone());
        let launcher = Launcher::new(reactor, host);
        let extraction_result: Result<T, _> = FromContext::from_context(&context);

        match extraction_result {
            Ok(arguments) => {
                let state = self.state.clone();
                let task = self
                    .configure
                    .call(launcher, arguments)
                    .then(|r| async move {
                        match r.into_handler_result() {
                            Ok(()) => *state.borrow_mut() = ConfigurationState::Finished,
                            Err(error) => {
                                log::error!("Launcher problem: {error}");
                                *state.borrow_mut() = ConfigurationState::Failed(error.into());
                            }
                        }
                    });
                let spawn_result = self.executor.borrow().spawner().spawn_local(task);
                if let Err(error) = spawn_result {
                    log::error!("Configuration problem: {error}");
                    *self.state.borrow_mut() = ConfigurationState::Failed(Rc::new(error));
                }
                self.executor.borrow_mut().run_until_stalled();
            }
            Err(extraction_error) => {
                let error: Box<dyn Error> = extraction_error.into();
                log::error!("Extraction problem in configuration: {error}");
                *self.state.borrow_mut() = ConfigurationState::Failed(error.into());
            }
        }
        true
    }

    fn on_tick(&mut self) {}
}
