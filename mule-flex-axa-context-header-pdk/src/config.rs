// Copyright 2023 Salesforce, Inc. All rights reserved.
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub issuer: String,

    #[serde(alias = "privateKey")]
    pub private_key: String,
    
    #[serde(alias = "audienceHeaderName")]
    pub audience_header_name: String
}
