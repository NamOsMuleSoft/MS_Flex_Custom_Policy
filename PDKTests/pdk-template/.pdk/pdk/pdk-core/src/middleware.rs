// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::host::property::{PropertyAccessor, TRACING_ID_PATH};
use classy::event::{EventData, RequestHeaders};
use classy::extract::FromContext;
use classy::BoxError;

pub fn for_request_headers(event: &EventData<RequestHeaders>) -> Result<(), BoxError> {
    load_request_id(event)
}

fn load_request_id(event: &EventData<RequestHeaders>) -> Result<(), BoxError> {
    let accessor: &dyn PropertyAccessor = FromContext::from_context(event)?;

    match accessor.tracing().id() {
        Ok(Some(_)) => {
            // Tracing ID already set up
        }
        _ => {
            // Storing Request ID in a custom property since, for an unknown reason, it is not available
            // in some HTTP events
            match accessor.request().id() {
                Ok(Some(id)) => accessor.set_property(TRACING_ID_PATH, id.as_bytes()),
                Ok(None) => log::debug!("Request id is not present"),
                Err(err) => log::debug!("Unexpected error retrieving request id: {}", err),
            }
        }
    };

    Ok(())
}
