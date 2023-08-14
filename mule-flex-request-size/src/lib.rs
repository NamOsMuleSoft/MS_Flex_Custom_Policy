use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use log::info;
use serde::{Deserialize, Serialize};

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
        Box::new(HttpConfigHeaderRoot {
            field_name: String::new(),
            max_body_size: 0,
        })
    });
}}

struct HttpConfigHeader {
    max_body_size: usize, // Store the maximum body size directly.
    current_body_size: usize, // Store the accumulated body size.
}

impl Context for HttpConfigHeader {}

// A simple struct to represent a JSON response message.
#[derive(Serialize, Deserialize)]
struct JsonResponse {
    message: String,
}

impl HttpContext for HttpConfigHeader {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        info!("on_http_request_headers");
        Action::Continue
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        info!("on_http_request_body");

        if self.current_body_size + _body_size > self.max_body_size {
            // If the accumulated body size exceeds the maximum allowed size, return BadRequest.
            info!("Received an HTTP request with a body size larger than the maximum allowed.");

            // Create a JSON response message.
            let json_response = JsonResponse {
                message: "Body size exceeds the maximum allowed.".to_string(),
            };

            // Convert the JSON response to a string.
            let response_body = serde_json::to_string(&json_response).unwrap();

            // Convert the JSON response to a byte slice (u8 slice).
            let response_body_bytes = response_body.as_bytes();

            // Send the HTTP response with the JSON message.
            self.send_http_response(401, Vec::new(), Some(response_body_bytes));

            // Reset the accumulated body size for the next request.
            self.current_body_size = 0;

            return Action::Pause;
        }

        // Update the accumulated body size.
        self.current_body_size += _body_size;

        if _end_of_stream {
            info!("Received the full HTTP request body.");
            // Reset the accumulated body size for the next request.
            self.current_body_size = 0;
        } else {
            info!("Received a part of the HTTP request body.");
        }

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        info!("on_http_response_headers");
        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        info!("on_http_response_body");
        Action::Continue
    }
}

#[derive(Serialize, Deserialize)]
struct PolicyConfig {
    #[serde(alias = "field-name")]
    field_name: String,
}

struct HttpConfigHeaderRoot {
    field_name: String,
    max_body_size: usize, // Store the maximum body size directly.
}

impl Context for HttpConfigHeaderRoot {}

impl RootContext for HttpConfigHeaderRoot {
    fn on_configure(&mut self, _: usize) -> bool {
        if let Some(config_bytes) = self.get_plugin_configuration() {
            let config: PolicyConfig = serde_json::from_slice(config_bytes.as_slice()).unwrap();
            self.field_name = config.field_name;
            self.max_body_size = self.field_name.parse::<usize>().unwrap() * 1024; // Initialize max_body_size once.
            info!("field name is {}", self.field_name);
        }
        true
    }

    fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(HttpConfigHeader {
            max_body_size: self.max_body_size, // Pass max_body_size to the HttpConfigHeader context.
            current_body_size: 0, // Initialize the accumulated body size to zero.
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}
