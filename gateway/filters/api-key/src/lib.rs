/// Parse a JSON config string and return the list of allowed API keys.
/// Returns an empty vec on empty input or parse failure.
pub fn parse_api_keys(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    // Minimal JSON parse: look for "api_keys":[...] without pulling in serde.
    // Format expected: {"api_keys":["key1","key2"]}
    parse_api_keys_from_json(s)
}

fn parse_api_keys_from_json(s: &str) -> Vec<String> {
    // Find the array value for "api_keys"
    let marker = "\"api_keys\"";
    let Some(marker_pos) = s.find(marker) else {
        return vec![];
    };
    let after_marker = &s[marker_pos + marker.len()..];
    // skip whitespace and ':'
    let after_colon = after_marker.trim_start().trim_start_matches(':').trim_start();
    // find opening '['
    let Some(open) = after_colon.find('[') else {
        return vec![];
    };
    let Some(close) = after_colon.find(']') else {
        return vec![];
    };
    let array_content = &after_colon[open + 1..close];
    // Split by ',' and strip quotes
    array_content
        .split(',')
        .filter_map(|item| {
            let trimmed = item.trim().trim_matches('"');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

/// Returns true if the given key is in the allowed list.
pub fn is_key_allowed(key: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|k| k == key)
}

#[cfg(target_arch = "wasm32")]
mod filter {
    use proxy_wasm::traits::{Context, HttpContext, RootContext};
    use proxy_wasm::types::{Action, ContextType, LogLevel};

    use crate::{is_key_allowed, parse_api_keys};

    proxy_wasm::main! {{
        proxy_wasm::set_log_level(LogLevel::Info);
        proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
            Box::new(ApiKeyRoot { allowed_keys: vec![] })
        });
    }}

    struct ApiKeyRoot {
        allowed_keys: Vec<String>,
    }

    impl Context for ApiKeyRoot {}

    impl RootContext for ApiKeyRoot {
        fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
            if let Some(bytes) = self.get_plugin_configuration() {
                let config_str = String::from_utf8_lossy(&bytes);
                self.allowed_keys = parse_api_keys(&config_str);
            }
            true
        }

        fn get_type(&self) -> Option<ContextType> {
            Some(ContextType::HttpContext)
        }

        fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
            Some(Box::new(ApiKeyFilter {
                allowed_keys: self.allowed_keys.clone(),
            }))
        }
    }

    struct ApiKeyFilter {
        allowed_keys: Vec<String>,
    }

    impl Context for ApiKeyFilter {}

    impl HttpContext for ApiKeyFilter {
        fn on_http_request_headers(&mut self, _: usize, _: bool) -> Action {
            let key = self.get_http_request_header("x-api-key");
            match key {
                Some(k) if is_key_allowed(&k, &self.allowed_keys) => Action::Continue,
                _ => {
                    self.send_http_response(
                        401,
                        vec![("content-type", "application/json")],
                        Some(b"{\"error\":\"Unauthorized\"}"),
                    );
                    Action::Pause
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_key_allowed, parse_api_keys};

    // --- Cycle 1: parse_api_keys ---

    #[test]
    fn empty_config_yields_no_keys() {
        let keys = parse_api_keys("");
        assert!(keys.is_empty());
    }

    #[test]
    fn valid_json_config_returns_keys() {
        let keys = parse_api_keys(r#"{"api_keys":["secret-key-1","secret-key-2"]}"#);
        assert_eq!(keys, vec!["secret-key-1", "secret-key-2"]);
    }

    #[test]
    fn malformed_json_yields_no_keys() {
        let keys = parse_api_keys("not-json");
        assert!(keys.is_empty());
    }

    #[test]
    fn single_key_config() {
        let keys = parse_api_keys(r#"{"api_keys":["only-key"]}"#);
        assert_eq!(keys, vec!["only-key"]);
    }

    // --- Cycle 2: is_key_allowed ---

    #[test]
    fn valid_key_in_list_is_allowed() {
        let allowed = vec!["secret-key-1".to_string(), "secret-key-2".to_string()];
        assert!(is_key_allowed("secret-key-1", &allowed));
    }

    #[test]
    fn unknown_key_is_rejected() {
        let allowed = vec!["secret-key-1".to_string()];
        assert!(!is_key_allowed("bad-key", &allowed));
    }

    #[test]
    fn empty_key_is_rejected() {
        let allowed = vec!["secret-key-1".to_string()];
        assert!(!is_key_allowed("", &allowed));
    }

    #[test]
    fn empty_allowed_list_rejects_everything() {
        let allowed: Vec<String> = vec![];
        assert!(!is_key_allowed("secret-key-1", &allowed));
    }
}
