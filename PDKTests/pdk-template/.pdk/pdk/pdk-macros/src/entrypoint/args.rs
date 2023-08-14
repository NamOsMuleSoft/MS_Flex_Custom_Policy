// Copyright 2023 Salesforce, Inc. All rights reserved.
use proc_macro2::{Ident, Span};
use std::collections::HashMap;
use std::convert::TryFrom;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{token, Error, Token};

#[derive(Debug, Clone)]
struct ConfigField {
    key: Ident,
    value: Ident,
}

impl Parse for ConfigField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        let _eq_token: token::Eq = input.parse()?;
        let value = input.parse()?;
        Ok(ConfigField { key, value })
    }
}

#[derive(Debug)]
pub struct Args {
    pub log_level: Ident,
}

impl TryFrom<HashMap<String, ConfigField>> for Args {
    type Error = Error;

    fn try_from(value: HashMap<String, ConfigField>) -> Result<Self, Self::Error> {
        let mut value = value;

        let log_level = remove_or_else(&mut value, "log_level", "Info");

        if let Some((name, tokens)) = value.iter().next() {
            return Err(Error::new(
                tokens.key.span(),
                format!("Unexpected token `{}`.", name),
            ));
        }

        Ok(Args { log_level })
    }
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parsed: HashMap<String, ConfigField> =
            Punctuated::<ConfigField, Token![,]>::parse_terminated(input)?
                .into_iter()
                .map(|item| (item.key.to_string(), item))
                .collect();

        Args::try_from(parsed)
    }
}

fn remove_or_else(hash: &mut HashMap<String, ConfigField>, name: &str, default: &str) -> Ident {
    hash.remove(name)
        .map(|field| field.value)
        .unwrap_or_else(|| Ident::new(default, Span::call_site()))
}
