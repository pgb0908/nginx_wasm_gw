/// Format a JSON log line from request/response metadata.
/// Extracted as a pure function so it can be tested on native targets.
pub fn format_log(method: &str, path: &str, status: &str) -> String {
    format!(
        r#"{{"method":"{method}","path":"{path}","status":{status}}}"#
    )
}

#[cfg(target_arch = "wasm32")]
mod filter {
    use super::format_log;
    use proxy_wasm::traits::{Context, HttpContext, RootContext};
    use proxy_wasm::types::{ContextType, LogLevel};

    proxy_wasm::main! {{
        proxy_wasm::set_log_level(LogLevel::Info);
        proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
            Box::new(LogRoot)
        });
    }}

    struct LogRoot;
    impl Context for LogRoot {}
    impl RootContext for LogRoot {
        fn get_type(&self) -> Option<ContextType> {
            Some(ContextType::HttpContext)
        }
        fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
            Some(Box::new(LogFilter {
                method: String::new(),
                path: String::new(),
            }))
        }
    }

    struct LogFilter {
        method: String,
        path: String,
    }

    impl Context for LogFilter {}

    impl HttpContext for LogFilter {
        fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> proxy_wasm::types::Action {
            self.method = self
                .get_http_request_header(":method")
                .unwrap_or_default();
            self.path = self
                .get_http_request_header(":path")
                .unwrap_or_default();
            proxy_wasm::types::Action::Continue
        }

        fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> proxy_wasm::types::Action {
            let status = self
                .get_http_response_header(":status")
                .unwrap_or_else(|| "0".to_string());
            let log_line = format_log(&self.method, &self.path, &status);
            let _ = proxy_wasm::hostcalls::log(LogLevel::Info, &log_line);
            proxy_wasm::types::Action::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_log;

    #[test]
    fn format_log_produces_valid_json_fields() {
        let result = format_log("GET", "/api/v1/test", "200");
        assert_eq!(result, r#"{"method":"GET","path":"/api/v1/test","status":200}"#);
    }

    #[test]
    fn format_log_handles_post_with_500() {
        let result = format_log("POST", "/api/v2/create", "500");
        assert_eq!(result, r#"{"method":"POST","path":"/api/v2/create","status":500}"#);
    }

    #[test]
    fn format_log_handles_empty_fields() {
        let result = format_log("", "", "0");
        assert_eq!(result, r#"{"method":"","path":"","status":0}"#);
    }
}
