// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::host::property::PropertyAccessor;
use crate::policy_context::static_policy_context_cache::StaticPolicyContextCache;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct LogMetadata {
    api_id: Rc<String>,
    policy_name: Rc<String>,
    req_id: Option<String>,
}

impl Clone for LogMetadata {
    fn clone(&self) -> Self {
        Self {
            api_id: Rc::clone(&self.api_id),
            policy_name: Rc::clone(&self.policy_name),
            req_id: self.req_id.clone(),
        }
    }
}

impl LogMetadata {
    fn load_metadata() -> LogMetadata {
        let metadata = StaticPolicyContextCache::read_metadata();

        let api_name = metadata
            .api_info()
            .map(|api| api.name().to_string())
            .unwrap_or_else(|| StaticPolicyContextCache::read_plugin_name_api_id().to_string());

        let policy_name = metadata.policy_id();
        let policy_namespace = metadata.policy_namespace();

        LogMetadata {
            api_id: Rc::new(api_name),
            policy_name: Rc::new(format!("{}.{}", policy_name, policy_namespace)),
            req_id: None,
        }
    }
}

impl<'a> From<&'a dyn PropertyAccessor> for LogMetadata {
    fn from(property_accessor: &'a dyn PropertyAccessor) -> Self {
        let mut metadata = Self::load_metadata();
        metadata.req_id = property_accessor.tracing().id().ok().flatten();
        metadata
    }
}

impl Display for LogMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.req_id {
            None => {
                let _ = f.write_fmt(format_args!(
                    "[policy: {}][api: {}]",
                    self.policy_name, self.api_id
                ));
            }
            Some(ref id) => {
                let _ = f.write_fmt(format_args!(
                    "[policy: {}][api: {}][req: {}]",
                    self.policy_name, self.api_id, &id
                ));
            }
        }
        Ok(())
    }
}
