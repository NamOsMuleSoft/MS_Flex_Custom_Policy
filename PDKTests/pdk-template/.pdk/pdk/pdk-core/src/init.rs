// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::log::configure_logger;
use crate::middleware::for_request_headers;
use crate::policy_context::static_policy_context_cache::StaticPolicyContextCache;
use classy::Plugin;

pub fn configure(_id: u32) -> Plugin {
    StaticPolicyContextCache::fresh_reload();
    configure_logger();
    configure_plugin()
}

fn configure_plugin() -> Plugin {
    Plugin::new().event_handler(for_request_headers)
}
