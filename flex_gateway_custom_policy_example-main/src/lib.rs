use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use serde::Deserialize;

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
}