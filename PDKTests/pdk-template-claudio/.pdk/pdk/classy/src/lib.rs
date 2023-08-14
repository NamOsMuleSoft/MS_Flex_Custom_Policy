// Copyright 2023 Salesforce, Inc. All rights reserved.
/// TODO W-11681503: Rustdocs
mod context;
mod entrypoint;
mod handler;
mod host;
mod reactor;
mod types;

pub mod bootstrap;
pub mod client;
pub mod event;
pub mod extract;
pub mod middleware;
pub mod plugin;

pub(crate) mod http_constants;
pub(crate) mod macros;

pub use entrypoint::Entrypoint;
pub use extract::config::Configuration;
pub use host::DefaultHost;
pub use host::Host;
pub use plugin::Plugin;
pub use proxy_wasm;

pub type BoxError = Box<dyn std::error::Error>;
