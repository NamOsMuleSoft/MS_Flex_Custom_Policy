// Copyright 2023 Salesforce, Inc. All rights reserved.
use serde::Deserialize;



#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(alias = "headerName")]
    pub header_name: String,

    #[serde(alias = "expectedHeaders")]
    pub expected_headers: u64
}