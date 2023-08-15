use jwt_simple::prelude::*;
use std::error::Error;


#[derive(Debug, Deserialize, Serialize)]
pub struct AccessTokenPayload {
    pub scope: String,
    pub client_id: String,
    pub iss: String,
    pub jti: String,
    #[serde(rename = "axa-department")]
    pub axa_department: String,
    pub sub: String,
    #[serde(rename = "preferredLanguage")]
    pub preferred_language: String,
    #[serde(rename = "axa-company")]
    pub axa_company: String,
    #[serde(rename = "axa-companyOU")]
    pub axa_company_ou: String,
    pub name: String,
    pub given_name: String,
    pub member_of: String,
    pub family_name: String,
    pub iat: i64,
    pub email: String,
    #[serde(rename = "axa-upn")]
    pub axa_upn: String,
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

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct JwtClaims {
    #[serde(rename = "iss")]
    pub issuer: String,
    
    #[serde(rename = "sub")]
    pub subject_id: String,
    
    #[serde(rename = "domain")]
    pub subject_domain: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "initialSub")]
    pub initial_subject: String,
    
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "domain")]
    pub initial_domain: String,
    
    #[serde(rename = "iat")]
    pub issued_at: u64,
    
    #[serde(rename = "exp")]
    pub expiration: u64,
    
    #[serde(rename = "customData")]
    pub custom_data: Option<CustomData>,
    
    #[serde(rename = "contextVersion")]
    pub context_version: String,
    
    pub initial_client_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "amr")]
    pub authentication_method: String,
    
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


impl JwtClaims {
    pub fn from_access_token_payloads(access_payload: AccessTokenPayload) -> Self {
        Self {
            issuer: "MS_FLEX".to_string(),
            subject_id: access_payload.sub.clone(),
            subject_domain: String::default(), // Set appropriately if needed
            initial_subject: String::default(), // Set appropriately if needed
            initial_domain: String::default(), // Set appropriately if needed
            issued_at: access_payload.iat as u64,
            expiration: access_payload.exp as u64,
            custom_data: Some(CustomData {
                scope: access_payload.scope
                // Initialize CustomData fields here
            }),
            context_version: "1.0".to_string(), // Set appropriately if needed
            initial_client_id: access_payload.client_id, // Set appropriately if needed
            authentication_method: String::default(), 
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
