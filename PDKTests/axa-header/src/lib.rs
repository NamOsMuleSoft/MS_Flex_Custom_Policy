// Copyright 2023 Salesforce, Inc. All rights reserved.

mod config;


use anyhow::Result;

use pdk::api::classy::bootstrap::Launcher;
use pdk::api::classy::event::{Exchange, HeadersAccessor, RequestHeaders};
use pdk::api::classy::Configuration;
use pdk::api::logger;
use crate::config::Config;




use jwt_simple::prelude::*;
use base64::decode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;


#[derive(Debug, Deserialize, Serialize)]
struct AccessTokenPayload {
    scope: String,
    client_id: String,
    iss: String,
    jti: String,
    #[serde(rename = "axa-department")]
    axa_department: String,
    sub: String,
    #[serde(rename = "preferredLanguage")]
    preferred_language: String,
    #[serde(rename = "axa-company")]
    axa_company: String,
    #[serde(rename = "axa-companyOU")]
    axa_company_ou: String,
    name: String,
    given_name: String,
    member_of: String,
    family_name: String,
    iat: i64,
    email: String,
    #[serde(rename = "axa-upn")]
    axa_upn: String,
    exp: i64,
}


#[derive(Debug, Deserialize, Serialize)]
struct CustomData {
    scope: String
}

#[derive(Debug, Deserialize, Serialize)]
struct JwtClaims {
    #[serde(rename = "iss")]
    issuer: String,
    #[serde(rename = "sub")]
    subject_id: String,
    #[serde(rename = "domain")]
    subject_domain: String,
    #[serde(rename = "initialSub")]
    initial_subject: String,
    #[serde(rename = "domain")]
    initial_domain: String,
    #[serde(rename = "iat")]
    issued_at: u64,
    #[serde(rename = "exp")]
    expiration: u64,
    #[serde(rename = "customData")]
    custom_data: Option<CustomData>,
    #[serde(rename = "contextVersion")]
    context_version: String,
    #[serde(rename = "initialClientId")]
    initial_client_id: String,
    #[serde(rename = "amr")]
    authentication_method: String,
}


fn decode_base64(input: &str) -> Result<String, Box<dyn Error>> {
    let decoded_bytes = base64::decode_config(input, base64::URL_SAFE)?;
    let decoded_string = String::from_utf8(decoded_bytes)?;
    Ok(decoded_string)
}

fn parse_jwt_payload(token: &str) -> Result<AccessTokenPayload, Box<dyn Error>> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format".into());
    }

    let encoded_payload = parts[1];
    let decoded_payload = decode_base64(encoded_payload)?;

    let payload: AccessTokenPayload = serde_json::from_str(&decoded_payload)?;

    Ok(payload)
}

fn create_jwt_claims_from_payloads(
    access_payload: AccessTokenPayload
) -> JwtClaims {
    JwtClaims {
        issuer: "MS_FLEX".to_string(),
        subject_id: access_payload.sub.clone(),
        subject_domain: "".to_string(), // Set appropriately if needed
        initial_subject: "".to_string(), // Set appropriately if needed
        initial_domain: "".to_string(), // Set appropriately if needed
        issued_at: access_payload.iat as u64,
        expiration: access_payload.exp as u64,
        custom_data: Some(CustomData {
            scope: access_payload.scope
            // Initialize CustomData fields here
        }),
        context_version: "1.0".to_string(), // Set appropriately if needed
        initial_client_id: access_payload.client_id, // Set appropriately if needed
        authentication_method: "".to_string(), // Set appropriately if needed
    }
}





// This filter shows how to log a specific request header.
// It uses the `header_name` and the `expected_headers` from the policy configuration
async fn filter(exchange: Exchange<RequestHeaders>, config: &Config) {
    //Once headers were received ask for them
    if let Some(event) = exchange.event_data() {
        //Obtain the header name from the config
        let header = &config.header_name;
        // Log the header value
        
        

        //NAM
        let header_value = event.header(header.as_str()).unwrap_or_default();
        logger::info!("Header value: {}", header_value);
        

        
        //code from old
        match parse_jwt_payload(&header_value) {
            Ok(decoded_payload) => {
                println!("{:#?}", decoded_payload);
        
                let jwt_claims = create_jwt_claims_from_payloads(decoded_payload);
        
                
                let claims = Claims::with_custom_claims(jwt_claims, Duration::from_hours(2));
        
        
        
        
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
-----END PRIVATE KEY-----".to_string();
        
        
        
        
            let key = RS256KeyPair::from_pem(&pem);
            println!("Key Created");
            match key {
                Ok(key) => {
                    println!("Key Created");
                    let token = key.sign(claims);
        
                    match token {
                        Ok(token) => {
                            println!("JWT Token: {}", token);

                            //self.add_http_request_header("X-AXA-CONTEXT", &token);
                            event.add_header("AXA-CONTEXT_HEADER", &token);

                        }
                        Err(err) => {
                            println!("Errorwith token: {}", err);
                        }
                       
                    }
                }
                Err(err) => {
                    println!("Error creating key: {}", err);
                }
            }
        
        
        
        
        
        
        
        
            }
            Err(err) => {
                eprintln!("Error parsing token: {}", err);
            }

        }}
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        //event.add_header("AXA-CONTEXT_HEADER", &header_value);




    }


#[pdk::api::entrypoint]
async fn configure(launcher: Launcher, Configuration(bytes): Configuration) -> Result<()> {
    let config = serde_json::from_slice(&bytes)?;
    launcher.launch(|e| filter(e, &config)).await?;
    Ok(())
}
