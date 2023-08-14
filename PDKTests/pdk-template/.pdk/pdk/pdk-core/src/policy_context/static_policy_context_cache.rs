// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::host::property::PropertyAccessor;
use crate::policy_context::metadata::{read_api_name_from_plugin_name, PolicyMetadata};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static ACTIVE_POLICY_METADATA: RefCell<Option<Rc<PolicyMetadata>>> = RefCell::new(None);
    static ACTIVE_PLUGIN_NAME_API_ID: RefCell<Option<Rc<String>>> = RefCell::new(None);
}

pub struct StaticPolicyContextCache;

impl StaticPolicyContextCache {
    pub fn fresh_reload() {
        let property_accessor = <dyn PropertyAccessor>::default();

        StaticPolicyContextCache::fix_metadata(&Rc::new(PolicyMetadata::from(property_accessor)));
        StaticPolicyContextCache::fix_plugin_name_api_id(&Rc::new(read_api_name_from_plugin_name(
            property_accessor,
        )))
    }

    pub fn fix_metadata(metadata: &Rc<PolicyMetadata>) {
        ACTIVE_POLICY_METADATA.with(|cell| cell.replace(Some(Rc::clone(metadata))));
    }

    pub fn fix_plugin_name_api_id(plugin_name_api_id: &Rc<String>) {
        ACTIVE_PLUGIN_NAME_API_ID.with(|cell| cell.replace(Some(Rc::clone(plugin_name_api_id))));
    }

    pub fn read_metadata() -> Rc<PolicyMetadata> {
        let metadata = ACTIVE_POLICY_METADATA
            .with(|cell| cell.borrow().clone())
            .unwrap_or_default();

        Rc::clone(&metadata)
    }

    pub fn read_plugin_name_api_id() -> Rc<String> {
        let metadata = ACTIVE_PLUGIN_NAME_API_ID
            .with(|cell| cell.borrow().clone())
            .unwrap_or_default();

        Rc::clone(&metadata)
    }
}
