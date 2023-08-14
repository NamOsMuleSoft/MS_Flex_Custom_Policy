// Copyright 2023 Salesforce, Inc. All rights reserved.
//! `Policy context related APIs` to access Flex policy data.
use crate::host::property::PropertyAccessor;
use crate::policy_context::authentication::AuthenticationHandler;
use crate::policy_context::metadata::PolicyMetadata;
use std::rc::Rc;

pub mod authentication;
pub mod metadata;
pub mod static_policy_context_cache;

const AUTHENTICATION_PROPERTY: &[&str] = &["authentication"];

/// An interface of the `Flex` `Policy Context`.
///
/// [`PolicyContext`] is responsible for
/// * Access policy metadata,
/// * Manage authentication data.
pub trait PolicyContext {
    /// Returns the policy metadata.
    fn policy_metadata(&self) -> Rc<PolicyMetadata>;

    /// Returns the authentication handler.
    fn authentication_handler(&self) -> &dyn AuthenticationHandler;

    /// Returns a property accessor
    fn connection_properties(&self) -> &dyn PropertyAccessor;
}

impl dyn PolicyContext {
    pub fn default() -> &'static dyn PolicyContext {
        &impls::Host
    }
}

mod impls {
    use super::{metadata::PolicyMetadata, PolicyContext};
    use crate::host::property::PropertyAccessor;
    use crate::policy_context::authentication::AuthenticationHandler;
    use crate::policy_context::static_policy_context_cache::StaticPolicyContextCache;
    use std::rc::Rc;

    pub(crate) struct Host;

    impl PolicyContext for Host {
        fn policy_metadata(&self) -> Rc<PolicyMetadata> {
            Rc::clone(&StaticPolicyContextCache::read_metadata())
        }

        fn authentication_handler(&self) -> &dyn AuthenticationHandler {
            <dyn AuthenticationHandler>::default()
        }

        fn connection_properties(&self) -> &dyn PropertyAccessor {
            <dyn PropertyAccessor>::default()
        }
    }
}
