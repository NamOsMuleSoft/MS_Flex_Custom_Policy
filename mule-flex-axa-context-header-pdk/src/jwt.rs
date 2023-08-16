use jwt_simple::prelude::*;
use std::{error::Error, time::{SystemTime, UNIX_EPOCH}};


#[derive(Debug, Deserialize, Serialize)]
pub struct AccessTokenPayload {
    pub scope: Option<String>,
    pub client_id: String,
    pub iss: String,
    pub jti: String,
    #[serde(rename = "axa-department")]
    pub axa_department: Option<String>,
    pub sub: String,
    #[serde(rename = "preferredLanguage")]
    pub preferred_language: Option<String>,
    #[serde(rename = "axa-company")]
    pub axa_company: Option<String>,
    #[serde(rename = "axa-companyOU")]
    pub axa_company_ou: Option<String>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub member_of: Option<String>,
    pub family_name: Option<String>,
    pub iat: Option<u64>,
    pub email: Option<String>,
    #[serde(rename = "axa-upn")]
    pub axa_upn: Option<String>,
    pub exp: i64,
    pub part_nr_ansp_person: Option<String>,
    #[serde(rename = "pi.sri")]
    pub pi_sri: Option<String>,
    pub part_nr_org: Option<String>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Actor {
    pub client_id: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JwtClaims {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    
    #[serde(rename = "iss")]
    pub issuer: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sub")]
    pub subject_id: Option<String>,
        
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iat")]
    pub issued_at: Option<u64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "exp")]
    pub expiration: Option<u64>,
    
    pub client_id: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "jti")]
    pub token_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_nr_ansp_person: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "pi.sri")]
    pub pi_sri: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_nr_org: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "aud")]
    pub audience: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "act")]
    pub actor: Option<Actor>
}

impl Default for JwtClaims {
    fn default() -> Self {

        let current_time = now_in_secs();
        let two_hours = current_time + 7200;

        Self { 
            scope: Default::default(),
            issuer: String::from("MS_FLEX"),
            subject_id: Default::default(),
            issued_at: Some(current_time),
            expiration: Some(two_hours),
            token_id: Some(uuid()),
            client_id: Default::default(),
            part_nr_ansp_person: Default::default(),
            pi_sri: Default::default(),
            part_nr_org: Default::default(),
            audience: Default::default(),
            actor: Default::default()
        }
    }
}


impl JwtClaims {
    pub fn from_access_token_payloads(access_payload: AccessTokenPayload) -> Self {
        
        let current_time = now_in_secs();
        let two_hours = current_time + 7200;

        Self {
            scope: access_payload.scope,
            issuer: "MS_FLEX".to_string(),
            subject_id: Some(access_payload.sub),
            issued_at: Some(current_time),
            expiration: Some(two_hours),
            token_id: Some(uuid()),
            client_id: access_payload.client_id, // Set appropriately if needed
            part_nr_ansp_person: access_payload.part_nr_ansp_person,
            pi_sri: access_payload.pi_sri,
            part_nr_org: access_payload.part_nr_org, // Set appropriately if needed
            audience: None,
            actor: None
        }
    }
}


impl AccessTokenPayload {
    pub fn parse_jwt_payload(token: &str) -> Result<Self, Box<dyn Error>> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid token format".into());
        }

        let encoded_payload = parts[1];
        let decoded_payload = decode_base64(encoded_payload)?;

        let payload: Self = serde_json::from_str(&decoded_payload)?;

        Ok(payload)
    }
}

pub fn decode_base64(input: &str) -> Result<String, Box<dyn Error>> {
    let decoded_bytes = base64::decode_config(input, base64::URL_SAFE)?;
    let decoded_string = String::from_utf8(decoded_bytes)?;
    Ok(decoded_string)
}

fn now_in_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn uuid() -> String {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap().as_nanos();

    let mut rng = oorandom::Rand64::new(seed);
    let mut bytes: [u8; 16] = [0; 16];
    for n in 0..16 {
        bytes[n] = rng.rand_u64() as u8;
    }
        
    uuid::Builder::from_bytes(bytes).into_uuid().to_string()
}

#[test]
fn test_uuid() {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let mut rng = oorandom::Rand64::new(seed);

    let mut bytes: [u8; 16] = [0; 16];
    
    for n in 0..16 {
        bytes[n] = rng.rand_u64() as u8;
    }
    
    let uuid = uuid::Builder::from_bytes(bytes).into_uuid().to_string();

    println!("{}", uuid);
}