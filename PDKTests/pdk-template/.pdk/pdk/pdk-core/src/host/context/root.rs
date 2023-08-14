// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::host::context::http::HttpContextAdapter;
use crate::host::property::PropertyAccessor;
use crate::policy_context::metadata::{read_api_name_from_plugin_name, PolicyMetadata};
use crate::policy_context::static_policy_context_cache::StaticPolicyContextCache;
use classy::proxy_wasm::traits::{Context, HttpContext, RootContext};
use classy::proxy_wasm::types::ContextType;
use std::rc::Rc;

pub struct RootContextAdapter {
    root_context: Box<dyn RootContext>,
    policy_metadata: Rc<PolicyMetadata>,
    plugin_name_api_id: Rc<String>,
}

impl RootContextAdapter {
    pub fn new(context: Box<dyn RootContext>) -> Self {
        let property_accessor = <dyn PropertyAccessor>::default();

        Self {
            root_context: context,
            policy_metadata: Rc::new(PolicyMetadata::from(property_accessor)),
            plugin_name_api_id: Rc::new(read_api_name_from_plugin_name(property_accessor)),
        }
    }

    pub fn boxed(self) -> Box<dyn RootContext> {
        Box::new(self)
    }

    fn fix_current_context(&self) {
        StaticPolicyContextCache::fix_metadata(&self.policy_metadata);
        StaticPolicyContextCache::fix_plugin_name_api_id(&self.plugin_name_api_id);
    }
}

impl Context for RootContextAdapter {
    fn on_http_call_response(
        &mut self,
        token_id: u32,
        num_headers: usize,
        body_size: usize,
        num_trailers: usize,
    ) {
        self.fix_current_context();
        self.root_context
            .on_http_call_response(token_id, num_headers, body_size, num_trailers)
    }

    fn on_done(&mut self) -> bool {
        self.fix_current_context();
        self.root_context.on_done()
    }
}

impl RootContext for RootContextAdapter {
    fn on_configure(&mut self, plugin_configuration_size: usize) -> bool {
        self.fix_current_context();
        self.root_context.on_configure(plugin_configuration_size)
    }

    fn create_http_context(&self, context_id: u32) -> Option<Box<dyn HttpContext>> {
        self.fix_current_context();
        self.root_context
            .create_http_context(context_id)
            .map(|ctx| {
                HttpContextAdapter::new(
                    ctx,
                    Rc::clone(&self.policy_metadata),
                    Rc::clone(&self.plugin_name_api_id),
                )
                .boxed()
            })
    }

    fn get_type(&self) -> Option<ContextType> {
        self.root_context.get_type()
    }
}
