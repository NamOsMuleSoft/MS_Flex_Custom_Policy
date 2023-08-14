// Copyright 2023 Salesforce, Inc. All rights reserved.
mod entrypoint;

use crate::entrypoint::generate_entrypoint;
use proc_macro2::TokenStream;

#[proc_macro_attribute]
pub fn entrypoint(
    metadata: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = TokenStream::from(input);
    let metadata = TokenStream::from(metadata);
    let output = match generate_entrypoint(metadata, input) {
        Ok(stream) => stream,
        Err(error) => error.to_compile_error(),
    };
    proc_macro::TokenStream::from(output)
}
