/// Header manipulation Wasm filter.
///
/// Reads add/remove rules from plugin configuration (inline JSON passed via
/// `proxy_wasm` directive) and applies them to both request and response
/// headers.  Priority 3 in the filter chain.  On any parse error the filter
/// continues without manipulation — it never blocks a request.

// ─── JSON parsing (no_std–friendly, no external crate) ───────────────────────

/// A minimal, allocation-based JSON parser that extracts the fields we care
/// about.  We avoid serde_json because wasm32-wasip1 builds under a `no_std`
/// runtime where many std features are unavailable.
pub mod config {
    use std::collections::HashMap;

    #[derive(Debug, Default, Clone)]
    pub struct DirectionRules {
        /// Headers to add (or overwrite).  key → value.
        pub add: HashMap<String, String>,
        /// Header names to remove.
        pub remove: Vec<String>,
    }

    #[derive(Debug, Default, Clone)]
    pub struct HeaderManipulationConfig {
        pub request: DirectionRules,
        pub response: DirectionRules,
    }

    impl HeaderManipulationConfig {
        /// Parse a JSON string of the form:
        /// ```json
        /// {
        ///   "request":  { "add": { "K": "V" }, "remove": ["K2"] },
        ///   "response": { "add": { "K": "V" }, "remove": ["K2"] }
        /// }
        /// ```
        /// Returns `None` if the input is not valid JSON or is missing the
        /// expected keys; callers treat `None` as "apply no rules".
        pub fn parse(json: &str) -> Option<Self> {
            let mut cfg = HeaderManipulationConfig::default();
            let json = json.trim();

            let request_val = extract_object(json, "request")?;
            cfg.request = parse_direction(request_val)?;

            let response_val = extract_object(json, "response")?;
            cfg.response = parse_direction(response_val)?;

            Some(cfg)
        }
    }

    // ── private helpers ──────────────────────────────────────────────────────

    /// Parse one direction block `{ "add": {...}, "remove": [...] }`.
    fn parse_direction(block: &str) -> Option<DirectionRules> {
        let mut rules = DirectionRules::default();

        if let Some(add_block) = extract_object(block, "add") {
            rules.add = parse_string_map(add_block)?;
        }
        if let Some(remove_block) = extract_array(block, "remove") {
            rules.remove = parse_string_array(remove_block)?;
        }
        Some(rules)
    }

    /// Find the value of a JSON key whose value is an object `{ … }`.
    /// Returns the slice including the braces.
    fn extract_object<'a>(json: &'a str, key: &str) -> Option<&'a str> {
        let needle = format!("\"{}\"", key);
        let start = json.find(needle.as_str())?;
        let after_key = &json[start + needle.len()..];
        let colon = after_key.find(':')? + 1;
        let rest = after_key[colon..].trim_start();
        if !rest.starts_with('{') {
            return None;
        }
        let end = matching_brace(rest)?;
        Some(&rest[..=end])
    }

    /// Find the value of a JSON key whose value is an array `[ … ]`.
    fn extract_array<'a>(json: &'a str, key: &str) -> Option<&'a str> {
        let needle = format!("\"{}\"", key);
        let start = json.find(needle.as_str())?;
        let after_key = &json[start + needle.len()..];
        let colon = after_key.find(':')? + 1;
        let rest = after_key[colon..].trim_start();
        if !rest.starts_with('[') {
            return None;
        }
        let end = matching_bracket(rest)?;
        Some(&rest[..=end])
    }

    /// Return the byte index of the closing `}` that matches the `{` at index 0.
    fn matching_brace(s: &str) -> Option<usize> {
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escape = false;
        for (i, ch) in s.char_indices() {
            if escape { escape = false; continue; }
            if ch == '\\' && in_string { escape = true; continue; }
            if ch == '"' { in_string = !in_string; continue; }
            if in_string { continue; }
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 { return Some(i); }
                }
                _ => {}
            }
        }
        None
    }

    /// Return the byte index of the closing `]` that matches the `[` at index 0.
    fn matching_bracket(s: &str) -> Option<usize> {
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escape = false;
        for (i, ch) in s.char_indices() {
            if escape { escape = false; continue; }
            if ch == '\\' && in_string { escape = true; continue; }
            if ch == '"' { in_string = !in_string; continue; }
            if in_string { continue; }
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 { return Some(i); }
                }
                _ => {}
            }
        }
        None
    }

    /// Parse `{ "key": "value", ... }` into a HashMap.
    fn parse_string_map(block: &str) -> Option<HashMap<String, String>> {
        let mut map = HashMap::new();
        // strip outer braces
        let inner = block.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
        if inner.is_empty() {
            return Some(map);
        }
        // iterate over `"key": "value"` pairs
        let mut rest = inner;
        while !rest.is_empty() {
            rest = rest.trim_start_matches([' ', '\n', '\r', '\t', ',']);
            if rest.is_empty() { break; }
            let (key, after_key) = parse_json_string(rest)?;
            let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
            let (val, after_val) = parse_json_string(after_colon)?;
            map.insert(key, val);
            rest = after_val;
        }
        Some(map)
    }

    /// Parse `[ "v1", "v2", ... ]` into a Vec.
    fn parse_string_array(block: &str) -> Option<Vec<String>> {
        let mut vec = Vec::new();
        let inner = block.trim().strip_prefix('[')?.strip_suffix(']')?.trim();
        if inner.is_empty() {
            return Some(vec);
        }
        let mut rest = inner;
        while !rest.is_empty() {
            rest = rest.trim_start_matches([' ', '\n', '\r', '\t', ',']);
            if rest.is_empty() { break; }
            let (val, after_val) = parse_json_string(rest)?;
            vec.push(val);
            rest = after_val;
        }
        Some(vec)
    }

    /// Parse one JSON string literal from the start of `s`.
    /// Returns `(unescaped_value, remaining_slice_after_closing_quote)`.
    fn parse_json_string(s: &str) -> Option<(String, &str)> {
        let s = s.trim_start();
        let s = s.strip_prefix('"')?;
        let mut result = String::new();
        let mut chars = s.char_indices();
        loop {
            let (i, ch) = chars.next()?;
            if ch == '\\' {
                let (_, esc) = chars.next()?;
                match esc {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    '/' => result.push('/'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    _ => { result.push('\\'); result.push(esc); }
                }
            } else if ch == '"' {
                return Some((result, &s[i + 1..]));
            } else {
                result.push(ch);
            }
        }
    }
}

// ─── Wasm filter (wasm32 only) ────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod filter {
    use super::config::HeaderManipulationConfig;
    use proxy_wasm::traits::{Context, HttpContext, RootContext};
    use proxy_wasm::types::{ContextType, LogLevel};

    proxy_wasm::main! {{
        proxy_wasm::set_log_level(LogLevel::Info);
        proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
            Box::new(HeaderManipulationRoot { config: None })
        });
    }}

    // ── Root context — reads config once on startup ──────────────────────────

    struct HeaderManipulationRoot {
        config: Option<HeaderManipulationConfig>,
    }

    impl Context for HeaderManipulationRoot {}

    impl RootContext for HeaderManipulationRoot {
        fn on_configure(&mut self, _plugin_configuration_size: usize) -> bool {
            if let Some(bytes) = self.get_plugin_configuration() {
                let json = String::from_utf8_lossy(&bytes);
                self.config = HeaderManipulationConfig::parse(&json);
            }
            true
        }

        fn get_type(&self) -> Option<ContextType> {
            Some(ContextType::HttpContext)
        }

        fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
            Some(Box::new(HeaderManipulation {
                config: self.config.clone(),
            }))
        }
    }

    // ── Per-request context ──────────────────────────────────────────────────

    struct HeaderManipulation {
        config: Option<HeaderManipulationConfig>,
    }

    impl Context for HeaderManipulation {}

    impl HttpContext for HeaderManipulation {
        fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool)
            -> proxy_wasm::types::Action
        {
            if let Some(cfg) = &self.config {
                for (k, v) in &cfg.request.add {
                    self.set_http_request_header(k, Some(v));
                }
                for k in &cfg.request.remove {
                    self.set_http_request_header(k, None);
                }
            }
            proxy_wasm::types::Action::Continue
        }

        fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool)
            -> proxy_wasm::types::Action
        {
            if let Some(cfg) = &self.config {
                for (k, v) in &cfg.response.add {
                    self.set_http_response_header(k, Some(v));
                }
                for k in &cfg.response.remove {
                    self.set_http_response_header(k, None);
                }
            }
            proxy_wasm::types::Action::Continue
        }
    }
}

// ─── Tests (native, no wasm32 toolchain needed) ───────────────────────────────

#[cfg(test)]
mod tests {
    use super::config::HeaderManipulationConfig;

    const SAMPLE_JSON: &str = r#"
    {
      "request": {
        "add":    { "X-Gateway": "nginx-wasm-gw", "X-Foo": "bar" },
        "remove": ["X-Internal-Secret", "X-Debug"]
      },
      "response": {
        "add":    { "X-Response-Time": "measured" },
        "remove": ["Server"]
      }
    }
    "#;

    #[test]
    fn parses_request_add_rules() {
        let cfg = HeaderManipulationConfig::parse(SAMPLE_JSON)
            .expect("parse should succeed");
        assert_eq!(cfg.request.add.get("X-Gateway").map(|s| s.as_str()), Some("nginx-wasm-gw"));
        assert_eq!(cfg.request.add.get("X-Foo").map(|s| s.as_str()), Some("bar"));
    }

    #[test]
    fn parses_request_remove_rules() {
        let cfg = HeaderManipulationConfig::parse(SAMPLE_JSON)
            .expect("parse should succeed");
        assert!(cfg.request.remove.contains(&"X-Internal-Secret".to_string()));
        assert!(cfg.request.remove.contains(&"X-Debug".to_string()));
    }

    #[test]
    fn parses_response_add_rules() {
        let cfg = HeaderManipulationConfig::parse(SAMPLE_JSON)
            .expect("parse should succeed");
        assert_eq!(
            cfg.response.add.get("X-Response-Time").map(|s| s.as_str()),
            Some("measured")
        );
    }

    #[test]
    fn parses_response_remove_rules() {
        let cfg = HeaderManipulationConfig::parse(SAMPLE_JSON)
            .expect("parse should succeed");
        assert!(cfg.response.remove.contains(&"Server".to_string()));
    }

    #[test]
    fn empty_add_and_remove_are_valid() {
        let json = r#"{"request":{"add":{},"remove":[]},"response":{"add":{},"remove":[]}}"#;
        let cfg = HeaderManipulationConfig::parse(json)
            .expect("empty rules should parse");
        assert!(cfg.request.add.is_empty());
        assert!(cfg.request.remove.is_empty());
    }

    #[test]
    fn invalid_json_returns_none() {
        assert!(HeaderManipulationConfig::parse("not json").is_none());
        assert!(HeaderManipulationConfig::parse("{}").is_none());
    }
}
