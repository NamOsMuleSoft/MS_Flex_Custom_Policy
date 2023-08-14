// Copyright 2023 Salesforce, Inc. All rights reserved.
mod args;

#[cfg(test)]
mod tests;

use args::Args;
use proc_macro2::TokenStream;
use quote::quote;
use std::iter::FromIterator;
use syn::{parse2, Error, ItemFn};

pub fn generate_entrypoint(
    metadata: TokenStream,
    input: TokenStream,
) -> Result<TokenStream, Error> {
    let annotated_fn = input.clone();
    let input_fn: ItemFn = parse2(input)?;
    let args: Args = parse2(metadata)?;

    let generated_code = emit_entrypoint(args, input_fn);

    Ok(TokenStream::from_iter([annotated_fn, generated_code]))
}

fn emit_entrypoint(args: Args, input_fn: ItemFn) -> proc_macro2::TokenStream {
    let _log_level = args.log_level; // Unused param left intentionally as an example
    let annotated_fn_name = input_fn.sig.ident;

    quote! {
        pdk::api::classy::proxy_wasm::main! {{
            pdk::api::classy::proxy_wasm::set_root_context(|context_id| {
                pdk::__internal::RootContextAdapter::new(
                    pdk::__internal::configure(context_id)
                        .entrypoint(#annotated_fn_name)
                        .create_root_context(context_id)
                ).boxed()
            });
        }}
    }
}
