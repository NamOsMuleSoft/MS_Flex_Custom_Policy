// Copyright 2023 Salesforce, Inc. All rights reserved.

mod config;
mod jwt;

use anyhow::Result;
use jwt_simple::prelude::{Claims, Duration, RS256KeyPair, RSAKeyPairLike, JWTClaims};
use log::info;
use pdk::api::classy::bootstrap::Launcher;
use pdk::api::classy::event::{Exchange, HeadersAccessor, RequestHeaders};
use pdk::api::classy::Configuration;
use pdk_core::classy::event::EventData;
use pdk_core::policy_context::PolicyContext;
use regex::Regex;
use crate::config::Config;
use crate::jwt::{AccessTokenPayload, JwtClaims};

const ACCESS_TOKEN_HEADER_NAME: &str = "access_token";
const API_KEY_HEADER_NAME: &str = "api-key";
const AXA_CONTEXT_HEADER_NAME: &str = "X-AXA-CONTEXT";


async fn filter(exchange: Exchange<RequestHeaders>, config: &Config) {

    let Some(event) = exchange.event_data() else { return };

    info!("Issuer {}", config.issuer);

    // Use cases 1, 3
    // process access token header
    let mut claims = process_access_token(&event);

    // *** Use case 2 not clear *** //

    // Use case 4
    // if the input has the api key header, set the client_id claim with it
    if let Some(api_key) = event.header(API_KEY_HEADER_NAME) {
        claims.custom.initial_client_id = api_key;
    }

    // Use case 5
    // If the input is client certificate, update the client_id claim with the cert subject
    update_with_mtls_context(&mut claims);

    // generate the axa-context token from resulting claims
    let token = generate_jwt(claims, &config.private_key);
    event.add_header(AXA_CONTEXT_HEADER_NAME, &token);

}


fn update_with_mtls_context(claims: &mut JWTClaims<JwtClaims>) {
    let conn_props = <dyn PolicyContext>::default().connection_properties();

    // if mTLS context is present then initialize the client_id with the cert subject CN
    if let Some(subject) = conn_props.read_property(&["connection","subject_peer_certificate"]) {
        claims.custom.initial_client_id = String::from_utf8(subject).unwrap();
    };
}

// generate claims payload from the request access token or default
fn process_access_token(event: &EventData<'_, RequestHeaders>) -> JWTClaims<JwtClaims> {

    // set the default duration for 2 hours
    let duration = Duration::from_hours(2);

    // try to get and parse the access token
    match event.header(ACCESS_TOKEN_HEADER_NAME) {
        // if the access token header is present
        Some(access_token) => {
            info!("Parsing the access token {}", access_token );
            
            // try to parse the jwt contained in the access token
            match AccessTokenPayload::parse_jwt_payload(&access_token) {
                Ok(decoded_payload) => {
                    info!("{:#?}", decoded_payload);
                    
                    // create a custom claims from access token attributes
                    let custom_claims = JwtClaims::from_access_token_payloads(decoded_payload);
                    info!("custom claims: {:?} " , custom_claims);

                    // create the claims object
                    Claims::with_custom_claims(custom_claims, duration)
                }
                Err(err) => {
                    info!("Error parsing token: {}", err);
                    
                    // create the default claims object
                    Claims::with_custom_claims(JwtClaims::default(), duration)
                }
            }
        },
        None => {
            info!("Access token not present");

            // create the default claims object
            Claims::with_custom_claims(JwtClaims::default(), duration)
        }
    }
}

// function to create the axa context jwt from the input claims and the provided private key
fn generate_jwt(claims: JWTClaims<JwtClaims>, private_key: &str) -> String {
    let pem = format_to_pem(private_key);

    match RS256KeyPair::from_pem(&pem) {
        Ok(key) => {
            info!("Key Created");
            let token = key.sign(claims);

            match token {
                Ok(token) => {
                    info!("JWT Token: {}", token);
                    token
                }
                Err(err) => {
                    panic!("Error with token: {}", err);
                }
            }
        }
        Err(err) => {
            panic!("Error creating key: {}", err);
        }
    }    
}

// idempotent function to format the private key pem from raw or pem format 
fn format_to_pem(private_key: &str) -> String {
    const PEM_HEADER: &str = "-----BEGIN PRIVATE KEY-----";
    const PEM_FOOTER: &str = "-----END PRIVATE KEY-----";
    const LINE_LENGTH: usize = 64;

	// remove heade, footer, lines, spaces, tabs to get the raw pk 
    let private_key = private_key.replace(PEM_HEADER, "").replace(PEM_FOOTER, "");
    let regex = Regex::new(r"[\n\s\t]").unwrap();
	let private_key = regex.replace_all(&private_key, "").to_string();

	// format private key as lines of 64 chars
    let regex = Regex::new(&format!("(.{{1,{}}})", LINE_LENGTH)).unwrap();
    let formatted_key = regex.replace_all(&private_key, "$1\n").to_string();

	// format the PEM with the header, content and footer and return
    format!("{}\n{}{}", PEM_HEADER, formatted_key, PEM_FOOTER)
}


// Policy entry point
#[pdk::api::entrypoint]
async fn configure(launcher: Launcher, Configuration(bytes): Configuration) -> Result<()> {
    let config = serde_json::from_slice(&bytes)?;
    launcher.launch(|e| filter(e, &config)).await?;
    Ok(())
}



#[test]
fn test_format_to_pem_from_raw_pk_content() {
    let pem = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCovMxQ0coFuxXf
Dd+72WN1D1nOxu4GOhPxARcfky7I5+NCHgAqw7a5sQo07Vv4XmLHLPuP2NFxN+sM
Qs94sdX2eEbhHahUhf+QT+Y5jDX7S+zTIcdCBYEoHrUBjnO+ZZhQTl2/d78ApCrS
1hNKteW3pxsWuzGG67A+cbCyvUx2WEgUiEuNNst9IShVCJjGyvWSO2Iwi83nyWNX
+UBge4dq8tTwnPk3H5Is1Dujd7uly/GbY4MZAqiQw5xSNqtzuwMPi7Br3YgBHmYA
i2zDCxK+h02oNZZ6QPmMtlMX+V/YrwwECxGqAz5Fhqx3OXoBSS082tkLWcvpUM0U
x7eEdbiDAgMBAAECggEAN07nI6iYMQexNBM+njqzzHdzJwFynKzlw23pj+p0/9pu
VcoyoNHU74nVwCJ7Lm51rzaR4IUfpZ5AF51Alx2ndenXxcssVUQ00C84Ve2c9hld
b5kXUI8wVh+2keOJEcQISG5fcTaFb2bgOIp9+VOlD+0gxnMmWUSg2N74HaZJzVI7
Uc7iE/p7u3Qp8xGY+l5CyiHuKBTASgdwdAlByDwtKFvayKxoImDxSUXz+l+2RpUV
DAArIy4CDnPb1hk1QLlUq9T0M4sbIyNNdZ/772FeijzdpW23O4lH8LUH+6uT0uBS
L5i4uxY9W6DfSBABEeScqB0Ts21nQ01AHJmThDRgUQKBgQDQLSPLyn3CneaCtNFB
StNwfM3f0BYLJxh2EIUWnKhEmVK6XMhEDRtDN96quwvAN6LMtos04EV2ZDDrpNC4
tGWnD4vGctISChC6AHKdLUYZFqCIxxjw2/WZ/+pT6CXcJk/g9LiWAS6d4hyyyT1p
qqKN2aApYYomrXI/e9PiKdL58QKBgQDPgEIpMg2Gevh9V8PKrTZEpgRUj5TzHxQu
A4jy7xFBd0A9asqoZDOo2hhOx/+rrTv+4geoMtOMxnws0E2ji9mZpVmJfzYUo9wk
iDjfm/+e8MzeS5gQ2j/2eE420ex2lp7pqLjOc2ZnmrOC/BrI7PqGeNILDngA/PbO
QxxEZsNFswKBgQCJb7O5QFcknGBpnHymEYNkOVEl2NgkxsvUbnWfBw/kMiE23jht
DQYZq5H6v4Azh8eYRU/EOehCEEVn3SjbOGYAFDhgbL+Zn0GJuu/wKsqjl5emlWM2
6NDNufH3MUWFgVmtF4OhrOgc3gG6WzeLQlcNNUcS6s1tWYcauGKbZqdd0QKBgEjH
CW19erTyKHl98NQDUIdfWyF1gp6LBf7lioD6TKkTdFqWPCI3ks7kP6ZSC2BhUCuj
h1/9A6naa+8j2DdGc7mp/u90yLkQh8Pga2IySsOqXZCSHvG6OrjtlTExC6jER1RY
swjl/MLVxaRpW9OhGnVTpwftuTVPhBjv/NgY2uB1AoGAQCqpbn374CU3NX1ll4gm
ByynBsL1MGWt3/RcNQoi9npkzTDY+XrFCguFMJcVUHFu931bXvRjNPwL9Kw10D1e
wVIXRfPThZaZmjZRSPiXpXfUU47WlOhStZ7btWhrgRHuntUf3GVT84gv0uHyOSMX
9jQaFGiEdHwjMC9eLuFwbL4=
-----END PRIVATE KEY-----";

    let raw = "MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCovMxQ0coFuxXfDd+72WN1D1nOxu4GOhPxARcfky7I5+NCHgAqw7a5sQo07Vv4XmLHLPuP2NFxN+sMQs94sdX2eEbhHahUhf+QT+Y5jDX7S+zTIcdCBYEoHrUBjnO+ZZhQTl2/d78ApCrS1hNKteW3pxsWuzGG67A+cbCyvUx2WEgUiEuNNst9IShVCJjGyvWSO2Iwi83nyWNX+UBge4dq8tTwnPk3H5Is1Dujd7uly/GbY4MZAqiQw5xSNqtzuwMPi7Br3YgBHmYAi2zDCxK+h02oNZZ6QPmMtlMX+V/YrwwECxGqAz5Fhqx3OXoBSS082tkLWcvpUM0Ux7eEdbiDAgMBAAECggEAN07nI6iYMQexNBM+njqzzHdzJwFynKzlw23pj+p0/9puVcoyoNHU74nVwCJ7Lm51rzaR4IUfpZ5AF51Alx2ndenXxcssVUQ00C84Ve2c9hldb5kXUI8wVh+2keOJEcQISG5fcTaFb2bgOIp9+VOlD+0gxnMmWUSg2N74HaZJzVI7Uc7iE/p7u3Qp8xGY+l5CyiHuKBTASgdwdAlByDwtKFvayKxoImDxSUXz+l+2RpUVDAArIy4CDnPb1hk1QLlUq9T0M4sbIyNNdZ/772FeijzdpW23O4lH8LUH+6uT0uBSL5i4uxY9W6DfSBABEeScqB0Ts21nQ01AHJmThDRgUQKBgQDQLSPLyn3CneaCtNFBStNwfM3f0BYLJxh2EIUWnKhEmVK6XMhEDRtDN96quwvAN6LMtos04EV2ZDDrpNC4tGWnD4vGctISChC6AHKdLUYZFqCIxxjw2/WZ/+pT6CXcJk/g9LiWAS6d4hyyyT1pqqKN2aApYYomrXI/e9PiKdL58QKBgQDPgEIpMg2Gevh9V8PKrTZEpgRUj5TzHxQuA4jy7xFBd0A9asqoZDOo2hhOx/+rrTv+4geoMtOMxnws0E2ji9mZpVmJfzYUo9wkiDjfm/+e8MzeS5gQ2j/2eE420ex2lp7pqLjOc2ZnmrOC/BrI7PqGeNILDngA/PbOQxxEZsNFswKBgQCJb7O5QFcknGBpnHymEYNkOVEl2NgkxsvUbnWfBw/kMiE23jhtDQYZq5H6v4Azh8eYRU/EOehCEEVn3SjbOGYAFDhgbL+Zn0GJuu/wKsqjl5emlWM26NDNufH3MUWFgVmtF4OhrOgc3gG6WzeLQlcNNUcS6s1tWYcauGKbZqdd0QKBgEjHCW19erTyKHl98NQDUIdfWyF1gp6LBf7lioD6TKkTdFqWPCI3ks7kP6ZSC2BhUCujh1/9A6naa+8j2DdGc7mp/u90yLkQh8Pga2IySsOqXZCSHvG6OrjtlTExC6jER1RYswjl/MLVxaRpW9OhGnVTpwftuTVPhBjv/NgY2uB1AoGAQCqpbn374CU3NX1ll4gmByynBsL1MGWt3/RcNQoi9npkzTDY+XrFCguFMJcVUHFu931bXvRjNPwL9Kw10D1ewVIXRfPThZaZmjZRSPiXpXfUU47WlOhStZ7btWhrgRHuntUf3GVT84gv0uHyOSMX9jQaFGiEdHwjMC9eLuFwbL4=";
    let result = format_to_pem(raw);

    assert_eq!(result, pem);

}

#[test]
fn test_format_to_pem_from_formated_pem() {
    let valid = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCovMxQ0coFuxXf
Dd+72WN1D1nOxu4GOhPxARcfky7I5+NCHgAqw7a5sQo07Vv4XmLHLPuP2NFxN+sM
Qs94sdX2eEbhHahUhf+QT+Y5jDX7S+zTIcdCBYEoHrUBjnO+ZZhQTl2/d78ApCrS
1hNKteW3pxsWuzGG67A+cbCyvUx2WEgUiEuNNst9IShVCJjGyvWSO2Iwi83nyWNX
+UBge4dq8tTwnPk3H5Is1Dujd7uly/GbY4MZAqiQw5xSNqtzuwMPi7Br3YgBHmYA
i2zDCxK+h02oNZZ6QPmMtlMX+V/YrwwECxGqAz5Fhqx3OXoBSS082tkLWcvpUM0U
x7eEdbiDAgMBAAECggEAN07nI6iYMQexNBM+njqzzHdzJwFynKzlw23pj+p0/9pu
VcoyoNHU74nVwCJ7Lm51rzaR4IUfpZ5AF51Alx2ndenXxcssVUQ00C84Ve2c9hld
b5kXUI8wVh+2keOJEcQISG5fcTaFb2bgOIp9+VOlD+0gxnMmWUSg2N74HaZJzVI7
Uc7iE/p7u3Qp8xGY+l5CyiHuKBTASgdwdAlByDwtKFvayKxoImDxSUXz+l+2RpUV
DAArIy4CDnPb1hk1QLlUq9T0M4sbIyNNdZ/772FeijzdpW23O4lH8LUH+6uT0uBS
L5i4uxY9W6DfSBABEeScqB0Ts21nQ01AHJmThDRgUQKBgQDQLSPLyn3CneaCtNFB
StNwfM3f0BYLJxh2EIUWnKhEmVK6XMhEDRtDN96quwvAN6LMtos04EV2ZDDrpNC4
tGWnD4vGctISChC6AHKdLUYZFqCIxxjw2/WZ/+pT6CXcJk/g9LiWAS6d4hyyyT1p
qqKN2aApYYomrXI/e9PiKdL58QKBgQDPgEIpMg2Gevh9V8PKrTZEpgRUj5TzHxQu
A4jy7xFBd0A9asqoZDOo2hhOx/+rrTv+4geoMtOMxnws0E2ji9mZpVmJfzYUo9wk
iDjfm/+e8MzeS5gQ2j/2eE420ex2lp7pqLjOc2ZnmrOC/BrI7PqGeNILDngA/PbO
QxxEZsNFswKBgQCJb7O5QFcknGBpnHymEYNkOVEl2NgkxsvUbnWfBw/kMiE23jht
DQYZq5H6v4Azh8eYRU/EOehCEEVn3SjbOGYAFDhgbL+Zn0GJuu/wKsqjl5emlWM2
6NDNufH3MUWFgVmtF4OhrOgc3gG6WzeLQlcNNUcS6s1tWYcauGKbZqdd0QKBgEjH
CW19erTyKHl98NQDUIdfWyF1gp6LBf7lioD6TKkTdFqWPCI3ks7kP6ZSC2BhUCuj
h1/9A6naa+8j2DdGc7mp/u90yLkQh8Pga2IySsOqXZCSHvG6OrjtlTExC6jER1RY
swjl/MLVxaRpW9OhGnVTpwftuTVPhBjv/NgY2uB1AoGAQCqpbn374CU3NX1ll4gm
ByynBsL1MGWt3/RcNQoi9npkzTDY+XrFCguFMJcVUHFu931bXvRjNPwL9Kw10D1e
wVIXRfPThZaZmjZRSPiXpXfUU47WlOhStZ7btWhrgRHuntUf3GVT84gv0uHyOSMX
9jQaFGiEdHwjMC9eLuFwbL4=
-----END PRIVATE KEY-----";

    let input = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCovMxQ0coFuxXf
Dd+72WN1D1nOxu4GOhPxARcfky7I5+NCHgAqw7a5sQo07Vv4XmLHLPuP2NFxN+sM
Qs94sdX2eEbhHahUhf+QT+Y5jDX7S+zTIcdCBYEoHrUBjnO+ZZhQTl2/d78ApCrS
1hNKteW3pxsWuzGG67A+cbCyvUx2WEgUiEuNNst9IShVCJjGyvWSO2Iwi83nyWNX
+UBge4dq8tTwnPk3H5Is1Dujd7uly/GbY4MZAqiQw5xSNqtzuwMPi7Br3YgBHmYA
i2zDCxK+h02oNZZ6QPmMtlMX+V/YrwwECxGqAz5Fhqx3OXoBSS082tkLWcvpUM0U
x7eEdbiDAgMBAAECggEAN07nI6iYMQexNBM+njqzzHdzJwFynKzlw23pj+p0/9pu
VcoyoNHU74nVwCJ7Lm51rzaR4IUfpZ5AF51Alx2ndenXxcssVUQ00C84Ve2c9hld
b5kXUI8wVh+2keOJEcQISG5fcTaFb2bgOIp9+VOlD+0gxnMmWUSg2N74HaZJzVI7
Uc7iE/p7u3Qp8xGY+l5CyiHuKBTASgdwdAlByDwtKFvayKxoImDxSUXz+l+2RpUV
DAArIy4CDnPb1hk1QLlUq9T0M4sbIyNNdZ/772FeijzdpW23O4lH8LUH+6uT0uBS
L5i4uxY9W6DfSBABEeScqB0Ts21nQ01AHJmThDRgUQKBgQDQLSPLyn3CneaCtNFB
StNwfM3f0BYLJxh2EIUWnKhEmVK6XMhEDRtDN96quwvAN6LMtos04EV2ZDDrpNC4
tGWnD4vGctISChC6AHKdLUYZFqCIxxjw2/WZ/+pT6CXcJk/g9LiWAS6d4hyyyT1p
qqKN2aApYYomrXI/e9PiKdL58QKBgQDPgEIpMg2Gevh9V8PKrTZEpgRUj5TzHxQu
A4jy7xFBd0A9asqoZDOo2hhOx/+rrTv+4geoMtOMxnws0E2ji9mZpVmJfzYUo9wk
iDjfm/+e8MzeS5gQ2j/2eE420ex2lp7pqLjOc2ZnmrOC/BrI7PqGeNILDngA/PbO
QxxEZsNFswKBgQCJb7O5QFcknGBpnHymEYNkOVEl2NgkxsvUbnWfBw/kMiE23jht
DQYZq5H6v4Azh8eYRU/EOehCEEVn3SjbOGYAFDhgbL+Zn0GJuu/wKsqjl5emlWM2
6NDNufH3MUWFgVmtF4OhrOgc3gG6WzeLQlcNNUcS6s1tWYcauGKbZqdd0QKBgEjH
CW19erTyKHl98NQDUIdfWyF1gp6LBf7lioD6TKkTdFqWPCI3ks7kP6ZSC2BhUCuj
h1/9A6naa+8j2DdGc7mp/u90yLkQh8Pga2IySsOqXZCSHvG6OrjtlTExC6jER1RY
swjl/MLVxaRpW9OhGnVTpwftuTVPhBjv/NgY2uB1AoGAQCqpbn374CU3NX1ll4gm
ByynBsL1MGWt3/RcNQoi9npkzTDY+XrFCguFMJcVUHFu931bXvRjNPwL9Kw10D1e
wVIXRfPThZaZmjZRSPiXpXfUU47WlOhStZ7btWhrgRHuntUf3GVT84gv0uHyOSMX
9jQaFGiEdHwjMC9eLuFwbL4=
-----END PRIVATE KEY-----";
    let result = format_to_pem(input);

    assert_eq!(result, valid);
}

#[test]
fn test_format_to_pem_from_formated_pem_with_tabs() {
    let valid = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCovMxQ0coFuxXf
Dd+72WN1D1nOxu4GOhPxARcfky7I5+NCHgAqw7a5sQo07Vv4XmLHLPuP2NFxN+sM
Qs94sdX2eEbhHahUhf+QT+Y5jDX7S+zTIcdCBYEoHrUBjnO+ZZhQTl2/d78ApCrS
1hNKteW3pxsWuzGG67A+cbCyvUx2WEgUiEuNNst9IShVCJjGyvWSO2Iwi83nyWNX
+UBge4dq8tTwnPk3H5Is1Dujd7uly/GbY4MZAqiQw5xSNqtzuwMPi7Br3YgBHmYA
i2zDCxK+h02oNZZ6QPmMtlMX+V/YrwwECxGqAz5Fhqx3OXoBSS082tkLWcvpUM0U
x7eEdbiDAgMBAAECggEAN07nI6iYMQexNBM+njqzzHdzJwFynKzlw23pj+p0/9pu
VcoyoNHU74nVwCJ7Lm51rzaR4IUfpZ5AF51Alx2ndenXxcssVUQ00C84Ve2c9hld
b5kXUI8wVh+2keOJEcQISG5fcTaFb2bgOIp9+VOlD+0gxnMmWUSg2N74HaZJzVI7
Uc7iE/p7u3Qp8xGY+l5CyiHuKBTASgdwdAlByDwtKFvayKxoImDxSUXz+l+2RpUV
DAArIy4CDnPb1hk1QLlUq9T0M4sbIyNNdZ/772FeijzdpW23O4lH8LUH+6uT0uBS
L5i4uxY9W6DfSBABEeScqB0Ts21nQ01AHJmThDRgUQKBgQDQLSPLyn3CneaCtNFB
StNwfM3f0BYLJxh2EIUWnKhEmVK6XMhEDRtDN96quwvAN6LMtos04EV2ZDDrpNC4
tGWnD4vGctISChC6AHKdLUYZFqCIxxjw2/WZ/+pT6CXcJk/g9LiWAS6d4hyyyT1p
qqKN2aApYYomrXI/e9PiKdL58QKBgQDPgEIpMg2Gevh9V8PKrTZEpgRUj5TzHxQu
A4jy7xFBd0A9asqoZDOo2hhOx/+rrTv+4geoMtOMxnws0E2ji9mZpVmJfzYUo9wk
iDjfm/+e8MzeS5gQ2j/2eE420ex2lp7pqLjOc2ZnmrOC/BrI7PqGeNILDngA/PbO
QxxEZsNFswKBgQCJb7O5QFcknGBpnHymEYNkOVEl2NgkxsvUbnWfBw/kMiE23jht
DQYZq5H6v4Azh8eYRU/EOehCEEVn3SjbOGYAFDhgbL+Zn0GJuu/wKsqjl5emlWM2
6NDNufH3MUWFgVmtF4OhrOgc3gG6WzeLQlcNNUcS6s1tWYcauGKbZqdd0QKBgEjH
CW19erTyKHl98NQDUIdfWyF1gp6LBf7lioD6TKkTdFqWPCI3ks7kP6ZSC2BhUCuj
h1/9A6naa+8j2DdGc7mp/u90yLkQh8Pga2IySsOqXZCSHvG6OrjtlTExC6jER1RY
swjl/MLVxaRpW9OhGnVTpwftuTVPhBjv/NgY2uB1AoGAQCqpbn374CU3NX1ll4gm
ByynBsL1MGWt3/RcNQoi9npkzTDY+XrFCguFMJcVUHFu931bXvRjNPwL9Kw10D1e
wVIXRfPThZaZmjZRSPiXpXfUU47WlOhStZ7btWhrgRHuntUf3GVT84gv0uHyOSMX
9jQaFGiEdHwjMC9eLuFwbL4=
-----END PRIVATE KEY-----";

    let input = "\n\t\t-----BEGIN PRIVATE KEY-----\n\t\tMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCovMxQ0coFuxXf\n\t\tDd+72WN1D1nOxu4GOhPxARcfky7I5+NCHgAqw7a5sQo07Vv4XmLHLPuP2NFxN+sM\n\t\tQs94sdX2eEbhHahUhf+QT+Y5jDX7S+zTIcdCBYEoHrUBjnO+ZZhQTl2/d78ApCrS\n\t\t1hNKteW3pxsWuzGG67A+cbCyvUx2WEgUiEuNNst9IShVCJjGyvWSO2Iwi83nyWNX\n\t\t+UBge4dq8tTwnPk3H5Is1Dujd7uly/GbY4MZAqiQw5xSNqtzuwMPi7Br3YgBHmYA\n\t\ti2zDCxK+h02oNZZ6QPmMtlMX+V/YrwwECxGqAz5Fhqx3OXoBSS082tkLWcvpUM0U\n\t\tx7eEdbiDAgMBAAECggEAN07nI6iYMQexNBM+njqzzHdzJwFynKzlw23pj+p0/9pu\n\t\tVcoyoNHU74nVwCJ7Lm51rzaR4IUfpZ5AF51Alx2ndenXxcssVUQ00C84Ve2c9hld\n\t\tb5kXUI8wVh+2keOJEcQISG5fcTaFb2bgOIp9+VOlD+0gxnMmWUSg2N74HaZJzVI7\n\t\tUc7iE/p7u3Qp8xGY+l5CyiHuKBTASgdwdAlByDwtKFvayKxoImDxSUXz+l+2RpUV\n\t\tDAArIy4CDnPb1hk1QLlUq9T0M4sbIyNNdZ/772FeijzdpW23O4lH8LUH+6uT0uBS\n\t\tL5i4uxY9W6DfSBABEeScqB0Ts21nQ01AHJmThDRgUQKBgQDQLSPLyn3CneaCtNFB\n\t\tStNwfM3f0BYLJxh2EIUWnKhEmVK6XMhEDRtDN96quwvAN6LMtos04EV2ZDDrpNC4\n\t\ttGWnD4vGctISChC6AHKdLUYZFqCIxxjw2/WZ/+pT6CXcJk/g9LiWAS6d4hyyyT1p\n\t\tqqKN2aApYYomrXI/e9PiKdL58QKBgQDPgEIpMg2Gevh9V8PKrTZEpgRUj5TzHxQu\n\t\tA4jy7xFBd0A9asqoZDOo2hhOx/+rrTv+4geoMtOMxnws0E2ji9mZpVmJfzYUo9wk\n\t\tiDjfm/+e8MzeS5gQ2j/2eE420ex2lp7pqLjOc2ZnmrOC/BrI7PqGeNILDngA/PbO\n\t\tQxxEZsNFswKBgQCJb7O5QFcknGBpnHymEYNkOVEl2NgkxsvUbnWfBw/kMiE23jht\n\t\tDQYZq5H6v4Azh8eYRU/EOehCEEVn3SjbOGYAFDhgbL+Zn0GJuu/wKsqjl5emlWM2\n\t\t6NDNufH3MUWFgVmtF4OhrOgc3gG6WzeLQlcNNUcS6s1tWYcauGKbZqdd0QKBgEjH\n\t\tCW19erTyKHl98NQDUIdfWyF1gp6LBf7lioD6TKkTdFqWPCI3ks7kP6ZSC2BhUCuj\n\t\th1/9A6naa+8j2DdGc7mp/u90yLkQh8Pga2IySsOqXZCSHvG6OrjtlTExC6jER1RY\n\t\tswjl/MLVxaRpW9OhGnVTpwftuTVPhBjv/NgY2uB1AoGAQCqpbn374CU3NX1ll4gm\n\t\tByynBsL1MGWt3/RcNQoi9npkzTDY+XrFCguFMJcVUHFu931bXvRjNPwL9Kw10D1e\n\t\twVIXRfPThZaZmjZRSPiXpXfUU47WlOhStZ7btWhrgRHuntUf3GVT84gv0uHyOSMX\n\t\t9jQaFGiEdHwjMC9eLuFwbL4=\n\t\t-----END PRIVATE KEY-----";

    let result = format_to_pem(input);

    assert_eq!(result, valid);
}
