// Copyright 2023 Salesforce, Inc. All rights reserved.

{% if useconfig -%}
mod config;

use anyhow::Result;
use pdk::api::classy::bootstrap::Launcher;
use pdk::api::classy::event::{Exchange, HeadersAccessor, RequestHeaders};
use pdk::api::classy::Configuration;
use pdk::api::logger;
use crate::config::Config;

// This filter shows how to log a specific request header.
// It uses the `header_name` and the `expected_headers` from the policy configuration
async fn filter(exchange: Exchange<RequestHeaders>, config: &Config) {
    //Once headers were received ask for them
    if let Some(event) = exchange.event_data() {
        //Obtain the header name from the config
        let header = &config.header_name;
        // Log the header value
        logger::info!("Header value: {}", event.header(header.as_str()).unwrap_or_default());
        if event.headers().len() == config.expected_headers as usize {
            logger::info!("Received {} headers, as expected.", config.expected_headers);
        } else {
            logger::info!("Received different headers than expected.");
        }
    }
}

#[pdk::api::entrypoint]
async fn configure(launcher: Launcher, Configuration(bytes): Configuration) -> Result<()> {
    let config = serde_json::from_slice(&bytes)?;
    launcher.launch(|e| filter(e, &config)).await?;
    Ok(())
}
{% else -%}
use pdk::api::classy::event::{Exchange, HeadersAccessor, RequestHeaders};
use pdk::api::logger;

// This filter shows how to log a specific request header.
#[pdk::api::entrypoint]
async fn filter(exchange: Exchange<RequestHeaders>) {
    //Once headers were received ask for them
    if let Some(event) = exchange.event_data() {
        // Log the header value
        logger::info!("Header value: {}", event.header("Token").unwrap_or_default());
    }
}
{% endif -%}
