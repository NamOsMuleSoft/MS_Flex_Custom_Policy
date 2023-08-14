// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::policy_context::metadata::PolicyMetadata;
use crate::policy_context::static_policy_context_cache::StaticPolicyContextCache;
use classy::proxy_wasm::traits::{Context, HttpContext};
use classy::proxy_wasm::types::Action;
use std::rc::Rc;

pub struct HttpContextAdapter {
    http_context: Box<dyn HttpContext>,
    policy_metadata: Rc<PolicyMetadata>,
    plugin_name_api_id: Rc<String>,
}

impl HttpContextAdapter {
    pub fn new(
        http_context: Box<dyn HttpContext>,
        policy_metadata: Rc<PolicyMetadata>,
        plugin_name_api_id: Rc<String>,
    ) -> Self {
        Self {
            http_context,
            policy_metadata,
            plugin_name_api_id,
        }
    }

    pub fn boxed(self) -> Box<dyn HttpContext> {
        Box::new(self)
    }

    fn fix_current_context(&self) {
        StaticPolicyContextCache::fix_metadata(&self.policy_metadata);
        StaticPolicyContextCache::fix_plugin_name_api_id(&self.plugin_name_api_id);
    }
}

impl Context for HttpContextAdapter {
    fn on_http_call_response(
        &mut self,
        token_id: u32,
        num_headers: usize,
        body_size: usize,
        num_trailers: usize,
    ) {
        self.fix_current_context();
        self.http_context
            .on_http_call_response(token_id, num_headers, body_size, num_trailers)
    }

    fn on_done(&mut self) -> bool {
        self.fix_current_context();
        self.http_context.on_done()
    }
}

impl HttpContext for HttpContextAdapter {
    fn on_http_request_headers(&mut self, num_headers: usize, end_of_stream: bool) -> Action {
        self.fix_current_context();
        self.http_context
            .on_http_request_headers(num_headers, end_of_stream)
    }

    fn on_http_response_headers(&mut self, num_headers: usize, end_of_stream: bool) -> Action {
        self.fix_current_context();
        self.http_context
            .on_http_response_headers(num_headers, end_of_stream)
    }
}
