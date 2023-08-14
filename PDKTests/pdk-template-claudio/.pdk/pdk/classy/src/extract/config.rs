// Copyright 2023 Salesforce, Inc. All rights reserved.
use super::{context::ConfigureContext, FromContext};
use std::convert::Infallible;

#[derive(Clone, Debug, Default, Hash)]
pub struct Configuration(pub Vec<u8>);

impl FromContext<ConfigureContext> for Configuration {
    type Error = Infallible;

    fn from_context(context: &ConfigureContext) -> Result<Self, Self::Error> {
        Ok(context
            .host
            .get_plugin_configuration()
            .map(Configuration)
            .unwrap_or_default())
    }
}
