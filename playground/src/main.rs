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

fn main() {


let access_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6IjZpS1Jvc2s1STFyZkxnLXM2Q3dJSGtLZllwcyIsInBpLmF0bSI6ImM1d3IifQ.eyJzY29wZSI6Im9wZW5pZCBwcm9maWxlIGVtYWlsIGNvbW11bml0aWVzIiwiY2xpZW50X2lkIjoibVBndGNSY0ZKbCIsImlzcyI6Imh0dHBzOi8vb25lbG9naW4uYXhhLmNvbSIsImp0aSI6Im16RWtrNXBWR0RFUjNkS0NaZk52bWFqYnlIRDhHWDQ1IiwiYXhhLWRlcGFydG1lbnQiOiJHT19HVE9fQiZER1BfSVNfQVBJIE1hbmFnZW1lbnQiLCJzdWIiOiJaOTI3U1kiLCJwcmVmZXJyZWRMYW5ndWFnZSI6IkVOIiwiYXhhLWNvbXBhbnkiOiJBWEEgR3JvdXAgT3BlcmF0aW9ucyBGcmFuY2UgLSBFeHRlcm5hbHMiLCJheGEtY29tcGFueU9VIjoiYXhhLWdyb3VwLW9wZXJhdGlvbnMtZnItZXh0IiwibmFtZSI6Ik5hbSBUb24gVGhhdCIsImdpdmVuX25hbWUiOiJOYW0iLCJtZW1iZXJfb2YiOiJheGF1c2VyIiwiZmFtaWx5X25hbWUiOiJUb24gVGhhdCIsImlhdCI6MTY5MTA0ODk3OCwiZW1haWwiOiJuYW0udG9uLXRoYXQuZXh0ZXJuYWxAYXhhLmNvbSIsImF4YS11cG4iOiJaOTI3U1lAbG9naW4uYXhhIiwiZXhwIjoxNjkxMDU2MTc4fQ.I1AjZ-BYmhqr9BOrefcNhUdUZ3-_IA0mg3Xde5TtMYl2SVx17V1z5JqLy5mKLzRShEBzrh5iPwGkH69F_5I0V5iWMEwkgkBHMbtTgCTL4S_q-gRKsrkg5hHbORe-tisszxFiHw8o9nCdvImX9aBWbrN9b_95ZrWairWSkCEFPXXXYBbx2PFdwNt9BNUOpvde1kEjMRpS85hoqqDtRT_rtIPO4oBeUYLEHOjVa-YtqATCt9stNNlE9RUZgY5BZrIEt65bxxl3dUkKV_XXyn5FX-3ATcdu7Y2pakpC6s-5nlsGp3_5uvSTirO2k0LbQGUky3_BZ54FdhPM2ITeGPSwfQ";


match parse_jwt_payload(access_token) {
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




    


    
    println!("DONE");




    }
    Err(err) => {
        eprintln!("Error parsing token: {}", err);
    }
}





 
    
 
    



}
