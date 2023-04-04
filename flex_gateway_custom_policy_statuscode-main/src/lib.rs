use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde::Deserialize;
use log::info;

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
    #[serde(alias = "property_name")]
    property_name: String,

    #[serde(alias = "secure_property_name")]
    secure_property_name: String,
}

// ---- CustomPolicyHeaderRoot ----

struct CustomPolicyHeaderRoot {
    pub config: CustomPolicyConfig,
}

impl Context for CustomPolicyHeaderRoot {}

impl RootContext for CustomPolicyHeaderRoot {
    fn on_configure(&mut self, _: usize) -> bool {
        info!("XXXOKOKOKZZZZZZZZZZZZZZZZZZZZZZZZXX");
        if let Some(config_bytes) = self.get_plugin_configuration() {
            self.config = serde_json::from_slice(config_bytes.as_slice()).unwrap()
        }
        true
    }

    fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
        info!("XXXOKOKOKZZZZZZZZZZZZZZZZZZZZZZZZXX");
        Some(Box::new(CustomPolicyHeader {
            config: self.config.clone()
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        info!("XXXOKOKOKZZZZZZZZZZZZZZZZZZZZZZZZXX");
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
        info!("on_http_response_header YYYY");
        self.add_http_response_header("Custom-Property", self.config.property_name.as_str());
        self.add_http_response_header("Secure-Custom-Property", self.config.secure_property_name.as_str());
        Action::Continue
    }

    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        match self.get_http_request_header(":path") {
            Some(path) if path == "/hello" => {
                self.send_http_response(
                    200,
                    vec![("Hello", "World"), ("Powered-By", "MuleSoft"), ("Custom-Property", self.config.property_name.as_str())],
                    Some(b"Hello, Custom Policy!\n"),
                );
                Action::Pause
            }
            _ => Action::Continue,
        }
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {

        if !_end_of_stream {
            // Wait -- we'll be called again when the complete body is buffered
            // at the host side.
            info!("on_http_response_body wait end of streamXXXXX");
            return Action::Pause;
        }
        
        if let Some(body_bytes) = self.get_http_response_body(0, _body_size) {
            info!("on_http_response_body wait read body");
            let body_str = String::from_utf8(body_bytes).unwrap();
            info!("XXXOKOKOKZZZZZZZZZZZZZZZZZZZZZZZZXX");
            info!("New body is {}",body_str);
            self.set_http_response_body(0, _body_size, &body_str.into_bytes());         
        }

        
          Action::Continue
    }
}