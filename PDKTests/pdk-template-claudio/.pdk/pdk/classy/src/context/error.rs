// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{error::Error, rc::Rc};

use proxy_wasm::{
    traits::{Context, HttpContext},
    types::Action,
};

use crate::Host;

pub struct ErrorContext {
    host: Rc<dyn Host>,
    error: Rc<dyn Error>,
}

impl ErrorContext {
    pub fn new(host: Rc<dyn Host>, error: Rc<dyn Error>) -> Self {
        Self { host, error }
    }

    fn on_http_request_event(&self) -> Action {
        log::warn!("failed to deploy policy {}", self.error);
        self.host.send_http_response(503, vec![], None);
        Action::Pause
    }
}

impl Context for ErrorContext {}

impl HttpContext for ErrorContext {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        self.on_http_request_event()
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        self.on_http_request_event()
    }

    fn on_http_request_trailers(&mut self, _num_trailers: usize) -> Action {
        self.on_http_request_event()
    }
}
