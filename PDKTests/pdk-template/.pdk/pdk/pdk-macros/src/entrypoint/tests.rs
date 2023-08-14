// Copyright 2023 Salesforce, Inc. All rights reserved.
use super::generate_entrypoint;
use proc_macro2::TokenStream;
use quote::quote;
use std::iter::FromIterator;

fn compare_token_streams(left: TokenStream, right: TokenStream) {
    assert_eq!(left.to_string(), right.to_string());
}

#[test]
pub fn test_entrypoint() {
    let metadata = quote! {log_level = Debug};
    let input = quote! {
        pub async fn my_configure(launcher: Launcher) -> Result<()> {
            launcher.launch(filter).await
        }
    };

    let added_code = quote! {
        pdk::api::classy::proxy_wasm::main! {{
            pdk::api::classy::proxy_wasm::set_root_context(|context_id| {
                pdk::__internal::RootContextAdapter::new(
                    pdk::__internal::configure(context_id)
                        .entrypoint(my_configure)
                        .create_root_context(context_id)
                ).boxed()
            });
        }}
    };

    let expected = TokenStream::from_iter([input.clone(), added_code]);
    let output = generate_entrypoint(metadata, input).unwrap();

    compare_token_streams(expected, output);
}
