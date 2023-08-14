// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    client::HttpCallResponse,
    event::{Event, EventData, EventKind, Exchange, RequestHeaders, ResponseHeaders},
    middleware::{EventHandlerDispatch, EventHandlerStack},
    reactor::{http::HttpReactor, root::RootReactor},
    types::{Cid, HttpCid},
    BoxError, Host,
};
use futures::executor::LocalPool;
use proxy_wasm::{
    traits::{Context, HttpContext},
    types::Action,
};

pub(crate) struct AsyncHttpContext {
    context_id: HttpCid,
    executor: Rc<RefCell<LocalPool>>,
    host: Rc<dyn Host>,
    config_reactor: Rc<RootReactor>,
    reactor: Rc<HttpReactor>,
    event_handlers: Rc<RefCell<EventHandlerStack>>,
}

impl AsyncHttpContext {
    pub fn new(
        context_id: HttpCid,
        executor: Rc<RefCell<LocalPool>>,
        host: Rc<dyn Host>,
        config_reactor: Rc<RootReactor>,
        reactor: Rc<HttpReactor>,
        event_handlers: Rc<RefCell<EventHandlerStack>>,
    ) -> Self {
        Self {
            context_id,
            executor,
            host,
            config_reactor,
            reactor,
            event_handlers,
        }
    }

    fn dispatch<S>(&self) -> Result<(), BoxError>
    where
        S: Event,
        EventHandlerStack: EventHandlerDispatch<S>,
    {
        let exchange: Exchange<S> = Exchange::new(self.reactor.clone(), self.host.clone());
        let event = EventData::new(&exchange);
        self.event_handlers.borrow_mut().dispatch(&event)
    }

    fn notify(&mut self, event: EventKind) -> Action {
        self.config_reactor
            .set_active_cid(Cid::Http(self.context_id));

        self.reactor.notify(event);

        let event_handler_result = match event {
            EventKind::RequestHeaders => self.dispatch::<RequestHeaders>(),
            EventKind::ResponseHeaders => self.dispatch::<ResponseHeaders>(),
            _ => Ok(()),
        };

        if let Err(err) = event_handler_result {
            log::error!("Failed event handler for {event:?}: {err:?}");
        }

        self.executor.borrow_mut().run_until_stalled();

        if self.reactor.paused() {
            Action::Pause
        } else {
            Action::Continue
        }
    }
}

impl Context for AsyncHttpContext {
    fn on_http_call_response(
        &mut self,
        token_id: u32,
        num_headers: usize,
        body_size: usize,
        num_trailers: usize,
    ) {
        self.config_reactor.notify_response(HttpCallResponse {
            request_id: token_id.into(),
            num_headers,
            body_size,
            num_trailers,
        });

        self.config_reactor.set_active_cid(self.context_id.into());
        self.executor.borrow_mut().run_until_stalled();
    }

    fn on_done(&mut self) -> bool {
        self.config_reactor.set_http_context_done(self.context_id);
        true
    }
}

impl HttpContext for AsyncHttpContext {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        self.notify(EventKind::RequestHeaders)
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        self.notify(EventKind::RequestBody)
    }

    fn on_http_request_trailers(&mut self, _num_trailers: usize) -> Action {
        self.notify(EventKind::RequestTrailers)
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        self.notify(EventKind::ResponseHeaders)
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        self.notify(EventKind::ResponseBody)
    }

    fn on_http_response_trailers(&mut self, _num_trailers: usize) -> Action {
        self.notify(EventKind::ResponseTrailers)
    }
}

impl Drop for AsyncHttpContext {
    fn drop(&mut self) {
        self.config_reactor.set_http_context_done(self.context_id);
    }
}
