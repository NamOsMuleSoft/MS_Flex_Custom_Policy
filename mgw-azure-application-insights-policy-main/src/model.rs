use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use crate::date_time::to_iso8601_utc;


#[derive(Default, Clone, Deserialize, Debug, Serialize)]
pub struct TrackRequest { 
    pub ver: i32,
    pub name: String,
    pub time: String,

    #[serde(rename = "iKey")]
    pub instrumentation_key: String,
    pub tags: Tags,
    pub data: TrackRequestData
}

#[derive(Default, Clone, Deserialize, Debug, Serialize)]
pub struct Tags {
    #[serde(rename = "ai.operation.id")]
    pub ai_operation_id: Option<String>,

    #[serde(rename = "ai.operation.name")]
    pub ai_operation_name: String,

    #[serde(rename = "ai.operation.parentId")]
    pub ai_operation_parent_id: Option<String>
}

#[derive(Default, Clone, Deserialize, Debug, Serialize)]
pub struct TrackRequestData {
    #[serde(rename = "baseType")]
    pub base_type: String,
    
    #[serde(rename = "baseData")]
    pub base_data: RequestData
}

#[derive(Default, Clone, Deserialize, Debug, Serialize)]
pub struct RequestData {
    pub ver: i32,
    pub id: String,
    pub name: String,
    pub duration: String,
    pub success: bool,

    #[serde(rename = "responseCode")]
    pub response_code: String,
    
    pub source: String,
    pub url: String
}


impl RequestData {
    
    pub fn from_request_headers(
        request_id: String,
        method: String, 
        scheme: String,
        authority: String,
        path: String,
        source: String) -> Self {
            
            Self { 
                ver: 2,
                id: request_id,
                name: format!("{} {}", method, path),
                duration: "00.00:00:00.000000".to_string(),
                success: false,
                response_code: String::default(),
                source,
            url: format!("{}://{}{}", scheme, authority, path)
         }         
    }
}


impl TrackRequest {
    pub fn new(time: SystemTime, instrumentation_key: String, request_data: RequestData, correlation_id: Option<String>, parent_id: Option<String>) -> Self {
        Self { 
            ver: 1, 
            name: "Microsoft.ApplicationInsights.Request".to_string(), 
            time: to_iso8601_utc(time),
            instrumentation_key,
            tags: Tags { 
                ai_operation_id: correlation_id.clone(),
                ai_operation_name: request_data.name.clone(), 
                ai_operation_parent_id: parent_id
            },
            data: TrackRequestData {
                base_type: "RequestData".to_string(),
                base_data: request_data.clone(),
            } 
        }
    }    
}

#[derive(Default, Clone, Deserialize, Debug, Serialize)]
pub struct TrackResponse {
    
    #[serde(rename = "itemsReceived")]
    pub items_received: i32,
    
    #[serde(rename = "itemsAccepted")]
    pub items_accepted: i32,
    
    pub errors: Vec<ErrorDetails>
    
}

#[derive(Default, Clone, Deserialize, Debug, Serialize)]
pub struct ErrorDetails {
    pub index: i32,
    
    #[serde(rename = "statusCode")]
    pub status_code: i32,
    
    pub message: String
}