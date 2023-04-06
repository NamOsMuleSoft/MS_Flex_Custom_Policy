mod tracking;
mod model;
mod date_time;

use log::debug;
use log::error;
use log::info;
use log::warn;
use model::RequestData;
use model::TrackResponse;
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde::Deserialize;
use std::time::Duration;

use crate::date_time::format_duration;
use crate::date_time::uuid;
use crate::tracking::AI_SERVICE_HOST_SUFFIX;
use crate::tracking::AI_SERVICE_NAME;
use crate::tracking::AI_SERVICE_PATH;
use crate::model::TrackRequest;


proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(PolicyRootContext {
            config: PolicyConfig::default(),
        })
    });
}}


struct PolicyRootContext {
    config: PolicyConfig,
}


#[derive(Default, Clone, Deserialize, Debug)]
struct PolicyConfig {
    #[serde(alias = "azureRegion")]
    azure_region: String,
    
    #[serde(alias = "apiKey")]
    api_key: String,
    
    #[serde(alias = "instrumentationKey")]
    instrumentation_key: String,

    #[serde(alias = "requestIdHeader")]
    request_id_header: String,

    #[serde(alias = "correlationIdHeader")]
    correlation_id_header: String
}

impl Context for PolicyRootContext {}

impl RootContext for PolicyRootContext {

    fn on_configure(&mut self, _: usize) -> bool {
        if let Some(config_bytes) = self.get_plugin_configuration() {
            self.config = serde_json::from_slice(config_bytes.as_slice()).unwrap()
        }
        info!("Policy configuration values: {:?}", self.config);
        true
    }

    fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(CustomHttpContext {
            config: self.config.clone(),
            correlation_id: None,
            traceparent: None,
            request_data: RequestData::default()
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}

struct CustomHttpContext {
    config: PolicyConfig,
    correlation_id: Option<String>,
    traceparent: Option<String>,
    request_data: RequestData
}


impl Context for CustomHttpContext {

    // Handler of the external service call
    fn on_http_call_response(&mut self, _: u32, _: usize, body_size: usize, _: usize) {
        
        // process the auth service response body
        if let Some(body) = self.get_http_call_response_body(0, body_size) {

            // get the http status code
            let response_status = self.get_http_call_response_header(":status").unwrap();
            

            // validate http status
            if response_status != "200" {
                // parse response body as raw string
                let payload: String = serde_json::from_slice(body.as_slice()).unwrap();
                error!("Azure Application Insights track request error: {:?}", payload);
            }
            else {
                // parse response body as TrackResponse
                let payload: TrackResponse = serde_json::from_slice(body.as_slice()).unwrap();
                debug!("Azure response payload: {:?}", payload);

                let rejected = payload.items_received - payload.items_accepted;
                if rejected != 0 {
                    warn!("{} tracking items rejected, errors: {:?} ", rejected, payload.errors);
                }
            }
        }
    }
}


impl HttpContext for CustomHttpContext {

    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        
        // gets the request id or generates a uuid
        let request_id_header = self.config.request_id_header.as_str();
        let request_id = match self.get_http_request_header(request_id_header) {
            Some(value) => value,
            None => uuid(self.get_current_time())
        };

        // propagates the current request-id as the upstream's traceparent
        self.set_http_request_header("traceparent", Some(&request_id));

        info!("Processing request id: {}", request_id);
        
        // gets the correlation id form the header
        let correlation_id_header = self.config.correlation_id_header.as_str();
        match self.get_http_request_header(correlation_id_header) {
            Some(value) => {
                // propagates the correlation-id to the upstream
                self.set_http_request_header(correlation_id_header, Some(&value));

                // keeps it to send to azure ai in the on_http_response_headers handler 
                self.correlation_id = Some(value);
            },
            None => {
                // sets the request-id as the correlation-id
                self.set_http_request_header(correlation_id_header, Some(&request_id));
            }
        };

        // get the traceparent
        match self.get_http_request_header("traceparent") {
            Some(value) => self.traceparent = Some(value),
            None => {},
        };

        // initializing the request data
        self.request_data = RequestData::from_request_headers(
            request_id.clone(),
            self.get_http_request_header(":method").unwrap(), 
            self.get_http_request_header(":scheme").unwrap(),
            self.get_http_request_header(":authority").unwrap(),
            self.get_http_request_header(":path").unwrap(),
            self.get_http_request_header("user-agent").unwrap_or("default".to_string())
        );

        debug!("Tracking request data initialized with: {:?}", self.request_data);

        Action::Continue

    }

    
    fn on_http_response_headers(&mut self, _: usize, _: bool) -> Action {

        info!("Processing response");

        // update response status code
        self.request_data.response_code = self.get_http_response_header(":status").unwrap();

        // update success from status < 300
        self.request_data.success = self.request_data.response_code.parse::<i32>().unwrap().cmp(&300).is_lt();

        // extracts the upstream service time to set the request duration
        let upstream_service_time: u64 = match self.get_http_response_header("x-envoy-upstream-service-time") {
            Some(value) => value.parse::<u64>().unwrap(),
            None => 0
        };

        // format as DD.HH:MM:SS.MMMMMM
        self.request_data.duration = format_duration(upstream_service_time);

        // get region prefix from configured region
        let region = self.config.azure_region.replace(" ", "").replace("(", "").replace(")", "").to_lowercase();
        
        // generates the authority from the configured region
        let authority = format!("{}.{}", region, AI_SERVICE_HOST_SUFFIX);

        // define http headers pairs
        let headers: Vec<(&str, &str)> = vec![
            (":method", "POST"),
            (":authority", &authority),
            (":path", AI_SERVICE_PATH),
            ("x-api-key", &self.config.api_key),
            ("content-type", "application/json")
        ];

        info!("Tracking request headers: {:?}", headers);

        let track_req = TrackRequest::new(
            self.get_current_time(),
            self.config.instrumentation_key.clone(),
            self.request_data.clone(),
            self.correlation_id.clone(),
            self.traceparent.clone()
        );
    
        let body = serde_json::to_string(&vec![&track_req]).unwrap();
        
        debug!("Track request body: {}", body);
        
        // sets the flex upstream service
        let upstream = format!("{}-{}.default.svc", AI_SERVICE_NAME, region);

        debug!("Azure App Insights upstream: {}", upstream);

        // request azure app insights upstream service
        match self.dispatch_http_call(
            &upstream,
            headers,
            Some(body.as_bytes()),
            vec![],
            Duration::from_secs(15)
        ){
            Ok(_) => {
                debug!("Tracking sent OK");
            },
            Err(err) => {
                error!("Error calling App Insights API: ({:?})", err);
            }
        }


        Action::Continue

    }

    
}
