use jwt_simple::prelude::*;
use std::error::Error;


#[derive(Debug, Deserialize, Serialize)]
pub struct AccessTokenPayload {
    pub scope: String,
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
pub struct CustomData {
    pub scope: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Actor {
    pub client_id: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JwtClaims {
    #[serde(rename = "iss")]
    pub issuer: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sub")]
    pub subject_id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "domain")]
    pub subject_domain: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "initialSub")]
    pub initial_subject: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "domain")]
    pub initial_domain: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iat")]
    pub issued_at: Option<u64>,
    
    #[serde(rename = "exp")]
    pub expiration: u64,
    
    #[serde(rename = "customData")]
    pub custom_data: Option<CustomData>,
    
    #[serde(rename = "contextVersion")]
    pub context_version: String,
    
    pub initial_client_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "amr")]
    pub authentication_method: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_nr_ansp_person: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "pi.sri")]
    pub pi_sri: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_nr_org: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "act")]
    pub actor: Option<Actor>
}

impl Default for JwtClaims {
    fn default() -> Self {
        Self { 
            issuer: String::from("MS_FLEX"),
            subject_id: Default::default(),
            subject_domain: Default::default(),
            initial_subject: Default::default(),
            initial_domain: Default::default(),
            issued_at: Default::default(),
            expiration: Default::default(),
            custom_data: Default::default(),
            context_version: String::from("1.0"),
            initial_client_id: Default::default(),
            authentication_method: Default::default(),
            part_nr_ansp_person: Default::default(),
            pi_sri: Default::default(),
            part_nr_org: Default::default(), 
            actor: Default::default() 
        }
    }
}


impl JwtClaims {
    pub fn from_access_token_payloads(access_payload: AccessTokenPayload) -> Self {
        Self {
            issuer: "MS_FLEX".to_string(),
            subject_id: Some(access_payload.sub.clone()),
            subject_domain: None, // Set appropriately if needed
            initial_subject: None, // Set appropriately if needed
            initial_domain: None, // Set appropriately if needed
            issued_at: access_payload.iat,
            expiration: access_payload.exp as u64,
            custom_data: Some(CustomData {
                scope: access_payload.scope
                // Initialize CustomData fields here
            }),
            context_version: "1.0".to_string(), // Set appropriately if needed
            initial_client_id: access_payload.client_id, // Set appropriately if needed
            authentication_method: None, 
            part_nr_ansp_person: access_payload.part_nr_ansp_person,
            pi_sri: access_payload.pi_sri,
            part_nr_org: access_payload.part_nr_org, // Set appropriately if needed
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
