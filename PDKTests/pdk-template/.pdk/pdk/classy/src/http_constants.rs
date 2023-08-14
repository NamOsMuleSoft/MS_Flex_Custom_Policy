// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::time::Duration;

pub const HEADER_METHOD: &str = ":method";
pub const HEADER_SCHEME: &str = ":scheme";
pub const HEADER_AUTHORITY: &str = ":authority";
pub const HEADER_PATH: &str = ":path";
pub const DEFAULT_PATH: &str = "/";
pub const HEADER_STATUS: &str = ":status";

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub const METHOD_POST: &str = "POST";
pub const METHOD_PUT: &str = "PUT";
pub const METHOD_GET: &str = "GET";
pub const METHOD_OPTIONS: &str = "OPTIONS";
pub const METHOD_DELETE: &str = "DELETE";
