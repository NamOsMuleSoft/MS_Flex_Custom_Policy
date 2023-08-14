use proxy_wasm::traits::*;
use proxy_wasm::types::*;

use log::info;


use base64::decode;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use jwt_simple::prelude::*;








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
struct IDTokenPayload {
    sub: String,
    aud: String,
    jti: String,
    iss: String,
    iat: i64,
    exp: i64,
    acr: String,
    auth_time: i64,
    #[serde(rename = "preferredLanguage")]
    preferred_language: String,
    name: String,
    given_name: String,
    member_of: Vec<String>,
    family_name: String,
    email: String,
    #[serde(rename = "pi.sri")]
    pi_sri: String,
}

#[derive(Debug, Serialize)]
struct CustomData {
    // ... (your CustomData fields)
}


#[derive(Debug, Serialize)]
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
    initial_client_id: Option<String>,
    #[serde(rename = "amr")]
    authentication_method: Option<String>,
}



enum PayloadType {
    AccessToken,
    IDToken,
}



fn base64_decode_and_parse<T: for<'a> Deserialize<'a>>(encoded: &str) -> Result<T, String> {
    let decoded_bytes = decode(encoded).map_err(|e| e.to_string())?;
    let decoded_str = String::from_utf8(decoded_bytes).map_err(|e| e.to_string())?;

    let payload: T = serde_json::from_str(&decoded_str).map_err(|e| e.to_string())?;
    Ok(payload)
}

fn extract_payload(token: &str, payload_type: PayloadType) -> Result<Value, String> {
    // Extract and decode the token
    let parts: Vec<&str> = token.split('.').collect();
    //if parts.len() < 3 {
    //    return Err(String::from("Invalid token format!"));
    //}

    let encoded_payload = parts[1];

    match payload_type {
        PayloadType::AccessToken => {
            // Call the function to parse the access token payload
            base64_decode_and_parse::<AccessTokenPayload>(encoded_payload)
                .map(|payload| serde_json::to_value(payload).unwrap())
        }
        PayloadType::IDToken => {
            // Call the function to parse the ID token payload
            base64_decode_and_parse::<IDTokenPayload>(encoded_payload)
                .map(|payload| serde_json::to_value(payload).unwrap())
        }
    }
}

fn create_jwt_claims_from_payloads(
    access_payload: (),
    id_payload: (),
) -> JwtClaims {
    JwtClaims {
        issuer: "MS_FLEX".to_string(),
        //subject_id: access_payload.sub.clone(),
        subject_id: "".to_string(),
        subject_domain: "".to_string(), // Set appropriately if needed
        initial_subject: "".to_string(), // Set appropriately if needed
        initial_domain: "".to_string(), // Set appropriately if needed
        //issued_at: access_payload.iat as u64,
        issued_at: "1629468154".parse().unwrap_or_default(),
        //expiration: access_payload.exp as u64,
        expiration: "1629468154".parse().unwrap_or_default(),
        custom_data: Some(CustomData {
            // Initialize CustomData fields here
        }),
        context_version: "".to_string(), // Set appropriately if needed
        initial_client_id: None, // Set appropriately if needed
        authentication_method: None, // Set appropriately if needed
    }
}

fn pretty_print_json<T: serde::Serialize>(data: &T) {
    if let Ok(pretty_data) = serde_json::to_string_pretty(data) {
        println!("{}", pretty_data);
    } else {
        println!("Error formatting JSON data");
    }
}












proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(CustomPolicyHeaderRoot {
            config: CustomPolicyConfig::default()
        })
    });
}}

// ---- CustomPolicyConfig ----

#[derive(Default, Clone, Deserialize)]
struct CustomPolicyConfig {
    #[serde(alias = "issuer")]
    issuer: String,

    #[serde(alias = "private_key")]
    private_key: String,
}

// ---- CustomPolicyHeaderRoot ----

struct CustomPolicyHeaderRoot {
    pub config: CustomPolicyConfig,
}

impl Context for CustomPolicyHeaderRoot {}

impl RootContext for CustomPolicyHeaderRoot {
    fn on_configure(&mut self, _: usize) -> bool {
        if let Some(config_bytes) = self.get_plugin_configuration() {
            self.config = serde_json::from_slice(config_bytes.as_slice()).unwrap()
        }
        true
    }

    fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(CustomPolicyHeader {
            config: self.config.clone()
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}

// ---- CustomPolicyHeader ----

struct CustomPolicyHeader {
    config: CustomPolicyConfig,
}

impl Context for CustomPolicyHeader {}

impl HttpContext for CustomPolicyHeader {
    fn on_http_response_headers(&mut self, _: usize, _: bool) -> Action {
        Action::Continue
    }

    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        info!("Issuer {}", self.config.issuer.as_str());
        info!("Private Key {}", self.config.private_key.as_str());





        let access_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6IjZpS1Jvc2s1STFyZkxnLXM2Q3dJSGtLZllwcyIsInBpLmF0bSI6ImM1d3IifQ.eyJzY29wZSI6Im9wZW5pZCBwcm9maWxlIGVtYWlsIGNvbW11bml0aWVzIiwiY2xpZW50X2lkIjoibVBndGNSY0ZKbCIsImlzcyI6Imh0dHBzOi8vb25lbG9naW4uYXhhLmNvbSIsImp0aSI6Im16RWtrNXBWR0RFUjNkS0NaZk52bWFqYnlIRDhHWDQ1IiwiYXhhLWRlcGFydG1lbnQiOiJHT19HVE9fQiZER1BfSVNfQVBJIE1hbmFnZW1lbnQiLCJzdWIiOiJaOTI3U1kiLCJwcmVmZXJyZWRMYW5ndWFnZSI6IkVOIiwiYXhhLWNvbXBhbnkiOiJBWEEgR3JvdXAgT3BlcmF0aW9ucyBGcmFuY2UgLSBFeHRlcm5hbHMiLCJheGEtY29tcGFueU9VIjoiYXhhLWdyb3VwLW9wZXJhdGlvbnMtZnItZXh0IiwibmFtZSI6Ik5hbSBUb24gVGhhdCIsImdpdmVuX25hbWUiOiJOYW0iLCJtZW1iZXJfb2YiOiJheGF1c2VyIiwiZmFtaWx5X25hbWUiOiJUb24gVGhhdCIsImlhdCI6MTY5MTA0ODk3OCwiZW1haWwiOiJuYW0udG9uLXRoYXQuZXh0ZXJuYWxAYXhhLmNvbSIsImF4YS11cG4iOiJaOTI3U1lAbG9naW4uYXhhIiwiZXhwIjoxNjkxMDU2MTc4fQ.I1AjZ-BYmhqr9BOrefcNhUdUZ3-_IA0mg3Xde5TtMYl2SVx17V1z5JqLy5mKLzRShEBzrh5iPwGkH69F_5I0V5iWMEwkgkBHMbtTgCTL4S_q-gRKsrkg5hHbORe-tisszxFiHw8o9nCdvImX9aBWbrN9b_95ZrWairWSkCEFPXXXYBbx2PFdwNt9BNUOpvde1kEjMRpS85hoqqDtRT_rtIPO4oBeUYLEHOjVa-YtqATCt9stNNlE9RUZgY5BZrIEt65bxxl3dUkKV_XXyn5FX-3ATcdu7Y2pakpC6s-5nlsGp3_5uvSTirO2k0LbQGUky3_BZ54FdhPM2ITeGPSwfQ";

        let id_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6IjZpS1Jvc2s1STFyZkxnLXM2Q3dJSGtLZllwcyJ9.eyJzdWIiOiJaOTI3U1kiLCJhdWQiOiJtUGd0Y1JjRkpsIiwianRpIjoiY24wamc5M0xpc0RDYVJDbkVyMU5GVyIsImlzcyI6Imh0dHBzOi8vb25lbG9naW4uYXhhLmNvbSIsImlhdCI6MTY5MTA0ODk3NiwiZXhwIjoxNjkxMDQ5Mjc2LCJhY3IiOiJ1cm46b2FzaXM6bmFtZXM6dGM6U0FNTDoyLjA6YWM6Y2xhc3NlczpQYXNzd29yZFByb3RlY3RlZFRyYW5zcG9ydCIsImF1dGhfdGltZSI6MTY5MTA0NjMwOCwicHJlZmVycmVkTGFuZ3VhZ2UiOiJFTiIsIm5hbWUiOiJOYW0gVG9uIFRoYXQiLCJnaXZlbl9uYW1lIjoiTmFtIiwibWVtYmVyX29mIjpbImF4YXVzZXIiXSwiZmFtaWx5X25hbWUiOiJUb24gVGhhdCIsImVtYWlsIjoibmFtLnRvbi10aGF0LmV4dGVybmFsQGF4YS5jb20iLCJwaS5zcmkiOiJhVDM3Yngwdi0tR1F4Rnp5b2E2LTR4U3JGd0kuWlhVdFkyVnVkSEpoYkMweC55MUdmIn0.fo3AxtvQAU_vj0F-ofnvvXQYchH2_vUMC3sXIWwy9K-EOr6eqq-ZXUl16pYsrG-76Tyu4_fPGrH2pLmhYKaQFwf08jmSFf-yUoGkSEiqmuUZwtkTyvnxH26-e2mbBA0sQOwbhNSpBO3UbC82IrZs1Pl6hdhvgrY3ldIXE0LCt8MixNI_I8_GW44Vv8yOZcTzVNc9Eqr6rLafRrkjMey6XpwdP8LifbcFC7iMD5GObcJr8kEjo7xXAzzD_hz8F4a1lr6DO8rJaxYx0_93uHn1Shy_RVPpv0tSNf6TD2ZJWVz9rCeH3GLBnxncbg30UQEHOTfUg2HMqszQ8FudhD2HGA";
        
    
    
       // let access_token = ""; // Replace with the actual access token value
        //let id_token = ""; // Replace with the actual ID token value
    
        let access_payload = match extract_payload(access_token, PayloadType::AccessToken) {
            Ok(Value::Object(access_token_payload)) => {
                serde_json::from_value(Value::Object(access_token_payload))
                    .map_err(|e| format!("Error parsing access token payload: {}", e))
                    .unwrap()
            }
            Err(err) => {
                eprintln!("Error extracting access token payload: {}", err);
                //return;
            }
            _ => {
                eprintln!("Unexpected payload format for access token");
                //return;
            }
        };



    
        let id_payload = match extract_payload(id_token, PayloadType::IDToken) {
            Ok(Value::Object(id_token_payload)) => {
                serde_json::from_value(Value::Object(id_token_payload))
                    .map_err(|e| format!("Error parsing ID token payload: {}", e))
                    .unwrap()
            }
            Err(err) => {
                eprintln!("Error extracting ID token payload: {}", err);
                //return;
            }
            _ => {
                eprintln!("Unexpected payload format for ID token");
                //return;
            }
        };


    
        
        // Pretty print the access_payload
        println!("ACCESS PAYLOAD");
        pretty_print_json(&access_payload);
    
         // Pretty print the id_payload
         println!("ID PAYLOAD");
         pretty_print_json(&id_payload);
      
    
    
    
        
        
    // Create JwtClaims from the payloads
    let jwt_claims = create_jwt_claims_from_payloads(access_payload, id_payload);
    
    
    // Pretty print the JwtClaims
    println!("JWT PAYLOAD");
    pretty_print_json(&jwt_claims);
    
    // Generate JWT header
    //let jwt_header = Header::new(Algorithm::RS256);
    
    
    
    
    
    let private_key = "-----BEGIN PRIVATE KEY-----
    MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC4Grx7wGZ6BmUd
    i7Vnz3KLb4hdGq6FpQvfvHD7j38ox50I5C4E5epEH8OYPNRX2NNlLbhIDuIowHrJ
    gjQ1OyHmbPgw8rsv9VP4zlcwh6iJEZeZiRUhVqs8zG5BoWOApI0RZSIKu9IZzVMq
    RsX2wCJy8nqg001OkLFNLQTlLUSf5sWYuVe3wSiZIH7BQyP/UqSchLjlmaKZGqkj
    QG5If+8itCxw8APYpH0WAKAfhCh5a5O/MqX7/3j1tZpShNKSlvlOo4j3hTprf4ms
    6aP/Y9lXA2KVy5BWMsEhfeOX6eZn+UIGsBa3bgg5t6UjD1Ci2rzsT+Bv+wvOY2PY
    HHxBgW2TAgMBAAECggEAIO6PZqeM7Sj1r2pNQcuG1tBVmSnrq/ZRAG2J3EXX3iiO
    u4dtf+zMkfEIoNPIvAoKDmA2RZV7+QQtMpx60EbKkphNcRwKRdLheXEXgZPSnuop
    rwopYHRLXy3GTzEaRmmo3kKz6y2bsIfc54QPZ0sJHnynWNtTSclyHbb+2CvXya5F
    Hzp6nC/23E8lS8dNZch3ZCEgiH07gqnQDZj6xHnvYy+ocPFz/lmPjllk1MEGd9iZ
    PwbmDsDbVj0xWiW60yyibkGMHiu92N9j8WlujYSvtkQlcXXj309KQPfzG2O6sKp9
    nDIaR/iX3bnbIBpAZTCX6usIkKX1AFOwkXK8l4+QSQKBgQDriDeW/rxjG4Rm3wZy
    Uz59Y2tluDcYO6ys8DZmzNR/DkkhGQSpkSCx2c6gaVW/UD2Y6XNYydyL6rWSdmwh
    rnDmYneUN75MdneA84u/TYI2Ps14GUJQb+8lJJV2MYU2elTlBuJWeAP8hk86K+xV
    4PTzI/fgyWa72s2y+ofKhzqchQKBgQDIGm4D7HoUu4/110L6t2pR4V3hQodWJ2ss
    oDQlR/ytDBNEG4qMfOUrQZcaJ01fEyTqT+yn8BoaBqSVpIS4pUTOys37Q0j4joV4
    4qczyoI/CV1AYbDPzby1Es5wbY8/ISF2uwWqiiqckDCxJtfZExFJNu3yEyTAq4cr
    2dFx7x6pNwKBgAW5nOhNrtSV4aUCfMygm18+4GhrjuNG6A6YFCpxhiTEeyCT1Bov
    DeVkzvH1PYFV+PlTi1s4JOU9wkYaHMzAybu/3vo6VKTVKFh5EweGYcjhw+rMamE8
    J0r21a82yu8lEBU1EqFZb3de6GQYlzkLK8kRMcBEBPxB+EgGcPCKUvFlAoGAA98f
    CcxlgEkwu9zyWs95qyPlIRqca7pPhImE+MOvy9lT9hliUN2JwZB/T+46vQAt0qrB
    NW6b0q0WOh74wmnQLwAVhCsFGmoHgxM/kOz2ICoaN8isFxkv8YDvpZU5FEubziRI
    M8iAko7nokXSH00TJIt/hxN/voqTDvIj70RlH3kCgYAJLkmYNgvrpDpRhiuk5zmV
    R6xhps1cp/hrcVPgjGs8/lQTqOAmkKyY0e7KfyBf1OieQTlD/2k3k+YG64q3vBp3
    WmGl/VnPYpFsnfpjgqdqPXXez0sbVnOnQ4qAjeAJ7L8BJBEgs0Oz+3Eb2v4I84MR
    wkEwEgoaWv1FSWPsuPWsTg==
    -----END PRIVATE KEY-----";
    
    
    let claims = Claims::create(Duration::from_hours(2));
    
 
    

    // Read the contents of the PEM-encoded private key file
    /*let pem_filename = "private_key.pem";
    let pem = match fs::read_to_string(pem_filename) {
        Ok(contents) => contents,
        Err(err) => {
            println!("Error reading PEM file: {}", err);
            return;
        }
    };
    
    println!("pem : {}", pem);
    */
    
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









        Action::Continue
    }
}