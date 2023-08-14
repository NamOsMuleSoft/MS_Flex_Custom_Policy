// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::time::{Duration, SystemTime};

use proxy_wasm::{
    hostcalls,
    types::{BufferType, Bytes, MapType, Status},
};

pub trait Host {
    fn get_current_time(&self) -> SystemTime;

    fn get_plugin_configuration(&self) -> Option<Bytes>;

    fn get_property(&self, path: Vec<&str>) -> Option<Bytes>;

    fn set_property(&self, path: Vec<&str>, value: Option<&[u8]>);

    fn get_shared_data(&self, key: &str) -> (Option<Bytes>, Option<u32>);

    fn set_shared_data(
        &self,
        key: &str,
        value: Option<&[u8]>,
        cas: Option<u32>,
    ) -> Result<(), Status>;

    fn register_shared_queue(&self, name: &str) -> u32;

    fn resolve_shared_queue(&self, vm_id: &str, name: &str) -> Option<u32>;

    fn dequeue_shared_queue(&self, queue_id: u32) -> Result<Option<Bytes>, Status>;

    fn enqueue_shared_queue(&self, queue_id: u32, value: Option<&[u8]>) -> Result<(), Status>;

    fn dispatch_http_call(
        &self,
        upstream: &str,
        headers: Vec<(&str, &str)>,
        body: Option<&[u8]>,
        trailers: Vec<(&str, &str)>,
        timeout: Duration,
    ) -> Result<u32, Status>;

    fn get_http_call_response_headers(&self) -> Vec<(String, String)>;

    fn get_http_call_response_headers_bytes(&self) -> Vec<(String, Bytes)>;

    fn get_http_call_response_header(&self, name: &str) -> Option<String>;

    fn get_http_call_response_header_bytes(&self, name: &str) -> Option<Bytes>;

    fn get_http_call_response_body(&self, start: usize, max_size: usize) -> Option<Bytes>;

    fn get_http_call_response_trailers(&self) -> Vec<(String, String)>;

    fn get_http_call_response_trailers_bytes(&self) -> Vec<(String, Bytes)>;

    fn get_http_call_response_trailer(&self, name: &str) -> Option<String>;

    fn get_http_call_response_trailer_bytes(&self, name: &str) -> Option<Bytes>;

    fn call_foreign_function(
        &self,
        function_name: &str,
        arguments: Option<&[u8]>,
    ) -> Result<Option<Bytes>, Status>;

    fn get_http_request_headers(&self) -> Vec<(String, String)>;

    fn get_http_request_headers_bytes(&self) -> Vec<(String, Bytes)>;

    fn set_http_request_headers(&self, headers: Vec<(&str, &str)>);

    fn set_http_request_headers_bytes(&self, headers: Vec<(&str, &[u8])>);

    fn get_http_request_header(&self, name: &str) -> Option<String>;

    fn get_http_request_header_bytes(&self, name: &str) -> Option<Bytes>;

    fn set_http_request_header(&self, name: &str, value: Option<&str>);

    fn set_http_request_header_bytes(&self, name: &str, value: Option<&[u8]>);

    fn add_http_request_header(&self, name: &str, value: &str);

    fn add_http_request_header_bytes(&self, name: &str, value: &[u8]);

    fn get_http_request_body(&self, start: usize, max_size: usize) -> Option<Bytes>;

    fn set_http_request_body(&self, start: usize, size: usize, value: &[u8]);

    fn get_http_request_trailers(&self) -> Vec<(String, String)>;

    fn get_http_request_trailers_bytes(&self) -> Vec<(String, Bytes)>;

    fn set_http_request_trailers(&self, trailers: Vec<(&str, &str)>);

    fn set_http_request_trailers_bytes(&self, trailers: Vec<(&str, &[u8])>);

    fn get_http_request_trailer(&self, name: &str) -> Option<String>;

    fn get_http_request_trailer_bytes(&self, name: &str) -> Option<Bytes>;

    fn set_http_request_trailer(&self, name: &str, value: Option<&str>);

    fn set_http_request_trailer_bytes(&self, name: &str, value: Option<&[u8]>);

    fn add_http_request_trailer(&self, name: &str, value: &str);

    fn add_http_request_trailer_bytes(&self, name: &str, value: &[u8]);

    fn resume_http_request(&self);

    fn get_http_response_headers(&self) -> Vec<(String, String)>;

    fn get_http_response_headers_bytes(&self) -> Vec<(String, Bytes)>;

    fn set_http_response_headers(&self, headers: Vec<(&str, &str)>);

    fn set_http_response_headers_bytes(&self, headers: Vec<(&str, &[u8])>);

    fn get_http_response_header(&self, name: &str) -> Option<String>;

    fn get_http_response_header_bytes(&self, name: &str) -> Option<Bytes>;

    fn set_http_response_header(&self, name: &str, value: Option<&str>);

    fn set_http_response_header_bytes(&self, name: &str, value: Option<&[u8]>);

    fn add_http_response_header(&self, name: &str, value: &str);

    fn add_http_response_header_bytes(&self, name: &str, value: &[u8]);

    fn get_http_response_body(&self, start: usize, max_size: usize) -> Option<Bytes>;

    fn set_http_response_body(&self, start: usize, size: usize, value: &[u8]);

    fn get_http_response_trailers(&self) -> Vec<(String, String)>;

    fn get_http_response_trailers_bytes(&self) -> Vec<(String, Bytes)>;

    fn set_http_response_trailers(&self, trailers: Vec<(&str, &str)>);

    fn set_http_response_trailers_bytes(&self, trailers: Vec<(&str, &[u8])>);

    fn get_http_response_trailer(&self, name: &str) -> Option<String>;

    fn get_http_response_trailer_bytes(&self, name: &str) -> Option<Bytes>;

    fn set_http_response_trailer(&self, name: &str, value: Option<&str>);

    fn set_http_response_trailer_bytes(&self, name: &str, value: Option<&[u8]>);

    fn add_http_response_trailer(&self, name: &str, value: &str);

    fn add_http_response_trailer_bytes(&self, name: &str, value: &[u8]);

    fn resume_http_response(&self);

    fn send_http_response(&self, status_code: u32, headers: Vec<(&str, &str)>, body: Option<&[u8]>);

    fn log(&self, level: proxy_wasm::types::LogLevel, message: &str);
}

pub struct DefaultHost;

fn unwrap_or_default<T: Default>(result: Result<T, Status>, function: &str) -> T {
    match result {
        Ok(value) => value,
        Err(e) => {
            log::warn!("Unhandled proxy-wasm error at DefaultHost::{function}(): {e:?}.");
            T::default()
        }
    }
}

// Gets the current function name
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);

        // Find and cut the rest of the path
        match &name[..name.len() - 3].rfind(':') {
            Some(pos) => &name[pos + 1..name.len() - 3],
            None => &name[..name.len() - 3],
        }
    }};
}

// Unwraps the result or default it and logs
macro_rules! unwrap_or_default {
    ($result:expr) => {{
        $crate::host::unwrap_or_default($result, function!())
    }};
}

impl Host for DefaultHost {
    fn get_current_time(&self) -> SystemTime {
        // Unable to use unwrap_or_default!() because
        // SystemTime does not implement Default
        hostcalls::get_current_time().expect("Current time")
    }

    fn get_plugin_configuration(&self) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_buffer(
            BufferType::PluginConfiguration,
            0,
            usize::MAX
        ))
    }

    fn get_property(&self, path: Vec<&str>) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_property(path))
    }

    fn set_property(&self, path: Vec<&str>, value: Option<&[u8]>) {
        unwrap_or_default!(hostcalls::set_property(path, value))
    }

    fn get_shared_data(&self, key: &str) -> (Option<Bytes>, Option<u32>) {
        unwrap_or_default!(hostcalls::get_shared_data(key))
    }

    fn set_shared_data(
        &self,
        key: &str,
        value: Option<&[u8]>,
        cas: Option<u32>,
    ) -> Result<(), proxy_wasm::types::Status> {
        hostcalls::set_shared_data(key, value, cas)
    }

    fn register_shared_queue(&self, name: &str) -> u32 {
        unwrap_or_default!(hostcalls::register_shared_queue(name))
    }

    fn resolve_shared_queue(&self, vm_id: &str, name: &str) -> Option<u32> {
        unwrap_or_default!(hostcalls::resolve_shared_queue(vm_id, name))
    }

    fn dequeue_shared_queue(&self, queue_id: u32) -> Result<Option<Bytes>, Status> {
        hostcalls::dequeue_shared_queue(queue_id)
    }

    fn enqueue_shared_queue(&self, queue_id: u32, value: Option<&[u8]>) -> Result<(), Status> {
        hostcalls::enqueue_shared_queue(queue_id, value)
    }

    fn dispatch_http_call(
        &self,
        upstream: &str,
        headers: Vec<(&str, &str)>,
        body: Option<&[u8]>,
        trailers: Vec<(&str, &str)>,
        timeout: std::time::Duration,
    ) -> Result<u32, Status> {
        hostcalls::dispatch_http_call(upstream, headers, body, trailers, timeout)
    }

    fn get_http_call_response_headers(&self) -> Vec<(String, String)> {
        unwrap_or_default!(hostcalls::get_map(MapType::HttpCallResponseHeaders))
    }

    fn get_http_call_response_headers_bytes(&self) -> Vec<(String, proxy_wasm::types::Bytes)> {
        unwrap_or_default!(hostcalls::get_map_bytes(MapType::HttpCallResponseHeaders))
    }

    fn get_http_call_response_header(&self, name: &str) -> Option<String> {
        unwrap_or_default!(hostcalls::get_map_value(
            MapType::HttpCallResponseHeaders,
            name
        ))
    }

    fn get_http_call_response_header_bytes(&self, name: &str) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_map_value_bytes(
            MapType::HttpCallResponseHeaders,
            name
        ))
    }

    fn get_http_call_response_body(&self, start: usize, max_size: usize) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_buffer(
            BufferType::HttpCallResponseBody,
            start,
            max_size
        ))
    }

    fn get_http_call_response_trailers(&self) -> Vec<(String, String)> {
        unwrap_or_default!(hostcalls::get_map(MapType::HttpCallResponseTrailers))
    }

    fn get_http_call_response_trailers_bytes(&self) -> Vec<(String, Bytes)> {
        unwrap_or_default!(hostcalls::get_map_bytes(MapType::HttpCallResponseTrailers))
    }

    fn get_http_call_response_trailer(&self, name: &str) -> Option<String> {
        unwrap_or_default!(hostcalls::get_map_value(
            MapType::HttpCallResponseTrailers,
            name
        ))
    }

    fn get_http_call_response_trailer_bytes(&self, name: &str) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_map_value_bytes(
            MapType::HttpCallResponseTrailers,
            name
        ))
    }

    fn call_foreign_function(
        &self,
        function_name: &str,
        arguments: Option<&[u8]>,
    ) -> Result<Option<Bytes>, proxy_wasm::types::Status> {
        hostcalls::call_foreign_function(function_name, arguments)
    }

    fn get_http_request_headers(&self) -> Vec<(String, String)> {
        unwrap_or_default!(hostcalls::get_map(MapType::HttpRequestHeaders))
    }

    fn get_http_request_headers_bytes(&self) -> Vec<(String, Bytes)> {
        unwrap_or_default!(hostcalls::get_map_bytes(MapType::HttpRequestHeaders))
    }

    fn set_http_request_headers(&self, headers: Vec<(&str, &str)>) {
        unwrap_or_default!(hostcalls::set_map(MapType::HttpRequestHeaders, headers))
    }

    fn set_http_request_headers_bytes(&self, headers: Vec<(&str, &[u8])>) {
        unwrap_or_default!(hostcalls::set_map_bytes(
            MapType::HttpRequestHeaders,
            headers
        ))
    }

    fn get_http_request_header(&self, name: &str) -> Option<String> {
        unwrap_or_default!(hostcalls::get_map_value(MapType::HttpRequestHeaders, name))
    }

    fn get_http_request_header_bytes(&self, name: &str) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_map_value_bytes(
            MapType::HttpRequestHeaders,
            name
        ))
    }

    fn set_http_request_header(&self, name: &str, value: Option<&str>) {
        unwrap_or_default!(hostcalls::set_map_value(
            MapType::HttpRequestHeaders,
            name,
            value
        ))
    }

    fn set_http_request_header_bytes(&self, name: &str, value: Option<&[u8]>) {
        unwrap_or_default!(hostcalls::set_map_value_bytes(
            MapType::HttpRequestHeaders,
            name,
            value
        ))
    }

    fn add_http_request_header(&self, name: &str, value: &str) {
        unwrap_or_default!(hostcalls::add_map_value(
            MapType::HttpRequestHeaders,
            name,
            value
        ))
    }

    fn add_http_request_header_bytes(&self, name: &str, value: &[u8]) {
        unwrap_or_default!(hostcalls::add_map_value_bytes(
            MapType::HttpRequestHeaders,
            name,
            value
        ))
    }

    fn get_http_request_body(&self, start: usize, max_size: usize) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_buffer(
            BufferType::HttpRequestBody,
            start,
            max_size
        ))
    }

    fn set_http_request_body(&self, start: usize, size: usize, value: &[u8]) {
        unwrap_or_default!(hostcalls::set_buffer(
            BufferType::HttpRequestBody,
            start,
            size,
            value
        ))
    }

    fn get_http_request_trailers(&self) -> Vec<(String, String)> {
        unwrap_or_default!(hostcalls::get_map(MapType::HttpRequestTrailers))
    }

    fn get_http_request_trailers_bytes(&self) -> Vec<(String, Bytes)> {
        unwrap_or_default!(hostcalls::get_map_bytes(MapType::HttpRequestHeaders))
    }

    fn set_http_request_trailers(&self, trailers: Vec<(&str, &str)>) {
        unwrap_or_default!(hostcalls::set_map(MapType::HttpRequestTrailers, trailers))
    }

    fn set_http_request_trailers_bytes(&self, trailers: Vec<(&str, &[u8])>) {
        unwrap_or_default!(hostcalls::set_map_bytes(
            MapType::HttpRequestTrailers,
            trailers
        ))
    }

    fn get_http_request_trailer(&self, name: &str) -> Option<String> {
        unwrap_or_default!(hostcalls::get_map_value(MapType::HttpRequestTrailers, name))
    }

    fn get_http_request_trailer_bytes(&self, name: &str) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_map_value_bytes(
            MapType::HttpRequestTrailers,
            name
        ))
    }

    fn set_http_request_trailer(&self, name: &str, value: Option<&str>) {
        unwrap_or_default!(hostcalls::set_map_value(
            MapType::HttpRequestTrailers,
            name,
            value
        ))
    }

    fn set_http_request_trailer_bytes(&self, name: &str, value: Option<&[u8]>) {
        unwrap_or_default!(hostcalls::set_map_value_bytes(
            MapType::HttpRequestTrailers,
            name,
            value
        ))
    }

    fn add_http_request_trailer(&self, name: &str, value: &str) {
        unwrap_or_default!(hostcalls::add_map_value(
            MapType::HttpRequestTrailers,
            name,
            value
        ))
    }

    fn add_http_request_trailer_bytes(&self, name: &str, value: &[u8]) {
        unwrap_or_default!(hostcalls::add_map_value_bytes(
            MapType::HttpRequestTrailers,
            name,
            value
        ))
    }

    fn resume_http_request(&self) {
        unwrap_or_default!(hostcalls::resume_http_request())
    }

    fn get_http_response_headers(&self) -> Vec<(String, String)> {
        unwrap_or_default!(hostcalls::get_map(MapType::HttpResponseHeaders))
    }

    fn get_http_response_headers_bytes(&self) -> Vec<(String, Bytes)> {
        unwrap_or_default!(hostcalls::get_map_bytes(MapType::HttpResponseHeaders))
    }

    fn set_http_response_headers(&self, headers: Vec<(&str, &str)>) {
        unwrap_or_default!(hostcalls::set_map(MapType::HttpResponseHeaders, headers))
    }

    fn set_http_response_headers_bytes(&self, headers: Vec<(&str, &[u8])>) {
        unwrap_or_default!(hostcalls::set_map_bytes(
            MapType::HttpResponseHeaders,
            headers
        ))
    }

    fn get_http_response_header(&self, name: &str) -> Option<String> {
        unwrap_or_default!(hostcalls::get_map_value(MapType::HttpResponseHeaders, name))
    }

    fn get_http_response_header_bytes(&self, name: &str) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_map_value_bytes(
            MapType::HttpResponseHeaders,
            name
        ))
    }

    fn set_http_response_header(&self, name: &str, value: Option<&str>) {
        unwrap_or_default!(hostcalls::set_map_value(
            MapType::HttpResponseHeaders,
            name,
            value
        ))
    }

    fn set_http_response_header_bytes(&self, name: &str, value: Option<&[u8]>) {
        unwrap_or_default!(hostcalls::set_map_value_bytes(
            MapType::HttpRequestHeaders,
            name,
            value
        ))
    }

    fn add_http_response_header(&self, name: &str, value: &str) {
        unwrap_or_default!(hostcalls::add_map_value(
            MapType::HttpResponseHeaders,
            name,
            value
        ))
    }

    fn add_http_response_header_bytes(&self, name: &str, value: &[u8]) {
        unwrap_or_default!(hostcalls::add_map_value_bytes(
            MapType::HttpResponseHeaders,
            name,
            value
        ))
    }

    fn get_http_response_body(&self, start: usize, max_size: usize) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_buffer(
            BufferType::HttpResponseBody,
            start,
            max_size
        ))
    }

    fn set_http_response_body(&self, start: usize, size: usize, value: &[u8]) {
        unwrap_or_default!(hostcalls::set_buffer(
            BufferType::HttpResponseBody,
            start,
            size,
            value
        ))
    }

    fn get_http_response_trailers(&self) -> Vec<(String, String)> {
        unwrap_or_default!(hostcalls::get_map(MapType::HttpResponseTrailers))
    }

    fn get_http_response_trailers_bytes(&self) -> Vec<(String, Bytes)> {
        unwrap_or_default!(hostcalls::get_map_bytes(MapType::HttpResponseTrailers))
    }

    fn set_http_response_trailers(&self, trailers: Vec<(&str, &str)>) {
        unwrap_or_default!(hostcalls::set_map(MapType::HttpResponseTrailers, trailers))
    }

    fn set_http_response_trailers_bytes(&self, trailers: Vec<(&str, &[u8])>) {
        unwrap_or_default!(hostcalls::set_map_bytes(
            MapType::HttpResponseTrailers,
            trailers
        ))
    }

    fn get_http_response_trailer(&self, name: &str) -> Option<String> {
        unwrap_or_default!(hostcalls::get_map_value(
            MapType::HttpResponseTrailers,
            name
        ))
    }

    fn get_http_response_trailer_bytes(&self, name: &str) -> Option<Bytes> {
        unwrap_or_default!(hostcalls::get_map_value_bytes(
            MapType::HttpResponseTrailers,
            name
        ))
    }

    fn set_http_response_trailer(&self, name: &str, value: Option<&str>) {
        unwrap_or_default!(hostcalls::set_map_value(
            MapType::HttpResponseTrailers,
            name,
            value
        ))
    }

    fn set_http_response_trailer_bytes(&self, name: &str, value: Option<&[u8]>) {
        unwrap_or_default!(hostcalls::set_map_value_bytes(
            MapType::HttpResponseTrailers,
            name,
            value
        ))
    }

    fn add_http_response_trailer(&self, name: &str, value: &str) {
        unwrap_or_default!(hostcalls::add_map_value(
            MapType::HttpResponseTrailers,
            name,
            value
        ))
    }

    fn add_http_response_trailer_bytes(&self, name: &str, value: &[u8]) {
        unwrap_or_default!(hostcalls::add_map_value_bytes(
            MapType::HttpResponseTrailers,
            name,
            value
        ))
    }

    fn resume_http_response(&self) {
        unwrap_or_default!(hostcalls::resume_http_response())
    }

    fn send_http_response(
        &self,
        status_code: u32,
        headers: Vec<(&str, &str)>,
        body: Option<&[u8]>,
    ) {
        unwrap_or_default!(hostcalls::send_http_response(status_code, headers, body))
    }

    fn log(&self, level: proxy_wasm::types::LogLevel, message: &str) {
        let _ = hostcalls::log(level, message);
    }
}

#[cfg(test)]
mod tests {
    use logtest::Logger;
    use proxy_wasm::types::Status;

    #[test]
    fn test_unwrap_or_default_ok() {
        let result = unwrap_or_default!(Ok(vec![1, 2]));

        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_unwrap_or_default_err() {
        let mut logger = Logger::start();

        fn foo() -> Vec<i32> {
            unwrap_or_default!(Err(Status::BadArgument))
        }

        let result = foo();

        assert_eq!(result, vec![]);
        assert_eq!(
            logger.pop().unwrap().args(),
            "Unhandled proxy-wasm error at DefaultHost::foo(): BadArgument."
        );
    }
}
