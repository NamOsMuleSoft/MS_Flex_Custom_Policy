// Copyright 2023 Salesforce, Inc. All rights reserved.
use classy::extract::FromContext;
use classy::proxy_wasm::types::Bytes;
use std::convert::Infallible;

pub use self::properties::*;
use anyhow::format_err;
use crate::host::{self};

mod properties;

pub trait PropertyAccessor {
    /// Returns a property, if not missing
    fn read_property(&self, path: &[&str]) -> Option<Bytes>;

    /// Overrides a given property with a given value
    fn set_property(&self, path: &[&str], value: &[u8]);
}

pub struct PropertyMapper<'a> {
    property_accessor: &'a dyn PropertyAccessor,
}

impl<'a>  PropertyMapper<'a>{
    fn string_property(&self, path: &[&str]) -> host::Result<Option<String>> {
        if let Some(bytes) = self.property_accessor.read_property(path) {
            String::from_utf8(bytes)
                .map(Option::from)
                .map_err(|e| format_err!("Retrieved value for property {:?} was not valid: {:?}", path, e))
        } else {
            Ok(None)
        }
    }

    pub fn from(property_accessor: &'a dyn PropertyAccessor) -> Self {
        Self { property_accessor }
    }
}

impl dyn PropertyAccessor {
    pub fn default() -> &'static dyn PropertyAccessor {
        &impls::Host
    }
}

impl<'a> dyn PropertyAccessor + 'a {
    pub fn request(&'a self) -> RequestInfo<'a> {
        RequestInfo {
            mapper: PropertyMapper::from(self)
        }
    }

    pub fn source(&'a self) -> SourceInfo<'a> {
        SourceInfo {
            mapper: PropertyMapper::from(self)
        }
    }

    pub fn destination(&'a self) -> DestinationInfo<'a> {
        DestinationInfo {
            mapper: PropertyMapper::from(self)
        }
    }

    pub fn tracing(&'a self) -> TracingInfo<'a> {
        TracingInfo {
            mapper: PropertyMapper::from(self)
        }
    }
}

pub struct RequestInfo<'a> {
    mapper: PropertyMapper<'a>,
}

impl<'a> RequestInfo<'a> {
    pub fn id(&self) -> host::Result<Option<String>> {
        self.mapper.string_property(REQUEST_ID)
    }

    pub fn protocol(&self) -> host::Result<Option<String>> {
        self.mapper.string_property(REQUEST_PROTOCOL)
    }

    pub fn scheme(&self) -> host::Result<Option<String>> {
        self.mapper.string_property(REQUEST_SCHEME)
    }
}

pub struct SourceInfo<'a> {
    mapper: PropertyMapper<'a>,
}

impl<'a> SourceInfo<'a> {
    pub fn address(&self) -> host::Result<Option<String>> {
        self.mapper.string_property(SOURCE_ADDRESS)
    }
}

pub struct DestinationInfo<'a> {
    mapper: PropertyMapper<'a>,
}

impl<'a> DestinationInfo<'a> {
    pub fn address(&self) -> host::Result<Option<String>> {
        self.mapper.string_property(DESTINATION_ADDRESS)
    }
}

pub struct TracingInfo<'a> {
    mapper: PropertyMapper<'a>,
}

impl<'a> TracingInfo<'a> {
    pub fn id(&self) -> host::Result<Option<String>> {
        self.mapper.string_property(TRACING_ID_PATH)
    }
}

impl<C> FromContext<C> for &'static dyn PropertyAccessor {
    type Error = Infallible;

    fn from_context(_: &C) -> Result<Self, Self::Error> {
        Ok(<dyn PropertyAccessor>::default())
    }
}

mod impls {
    use classy::proxy_wasm::types::Bytes;
    use classy::Host as ClassyHost;
    use crate::host::property::PropertyAccessor;

    pub(super) struct Host;

    impl PropertyAccessor for Host {
        fn read_property(&self, path: &[&str]) -> Option<Bytes> {
            crate::Host.get_property(path.to_vec())
        }

        fn set_property(&self, path: &[&str], value: &[u8]) {
            crate::Host.set_property(path.to_vec(), Some(value))
        }
    }
}
