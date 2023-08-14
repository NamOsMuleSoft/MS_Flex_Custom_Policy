// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{convert::Infallible, rc::Rc};

use crate::{
    reactor::{http::HttpReactor, root::RootReactor},
    types::{HttpCid, RootCid},
    Host,
};

use super::FromContext;

pub struct ConfigureContext {
    pub(crate) host: Rc<dyn Host>,
    pub(crate) root_reactor: Rc<RootReactor>,
}

impl ConfigureContext {
    pub(crate) fn new(host: Rc<dyn Host>, root_reactor: Rc<RootReactor>) -> Self {
        Self { host, root_reactor }
    }
}

impl FromContext<ConfigureContext> for Rc<dyn Host> {
    type Error = Infallible;

    fn from_context(context: &ConfigureContext) -> Result<Self, Self::Error> {
        Ok(context.host.clone())
    }
}

impl FromContext<ConfigureContext> for Rc<RootReactor> {
    type Error = Infallible;

    fn from_context(context: &ConfigureContext) -> Result<Self, Self::Error> {
        Ok(context.root_reactor.clone())
    }
}

impl FromContext<ConfigureContext> for RootCid {
    type Error = Infallible;

    fn from_context(context: &ConfigureContext) -> Result<Self, Self::Error> {
        Ok(context.root_reactor.context_id())
    }
}

pub struct FilterContext {
    host: Rc<dyn Host>,
    root_reactor: Rc<RootReactor>,
    http_reactor: Rc<HttpReactor>,
}

impl FilterContext {
    pub(crate) fn new(
        host: Rc<dyn Host>,
        root_reactor: Rc<RootReactor>,
        http_reactor: Rc<HttpReactor>,
    ) -> Self {
        Self {
            host,
            root_reactor,
            http_reactor,
        }
    }
}

impl FromContext<FilterContext> for Rc<dyn Host> {
    type Error = Infallible;

    fn from_context(context: &FilterContext) -> Result<Self, Self::Error> {
        Ok(context.host.clone())
    }
}

impl FromContext<FilterContext> for Rc<RootReactor> {
    type Error = Infallible;

    fn from_context(context: &FilterContext) -> Result<Self, Self::Error> {
        Ok(context.root_reactor.clone())
    }
}

impl FromContext<FilterContext> for Rc<HttpReactor> {
    type Error = Infallible;

    fn from_context(context: &FilterContext) -> Result<Self, Self::Error> {
        Ok(context.http_reactor.clone())
    }
}

impl FromContext<FilterContext> for RootCid {
    type Error = Infallible;

    fn from_context(context: &FilterContext) -> Result<Self, Self::Error> {
        Ok(context.root_reactor.context_id())
    }
}

impl FromContext<FilterContext> for HttpCid {
    type Error = Infallible;

    fn from_context(context: &FilterContext) -> Result<Self, Self::Error> {
        Ok(context.http_reactor.context_id())
    }
}
