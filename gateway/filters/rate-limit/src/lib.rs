// Rate Limiting Wasm Filter
// API Key 기준으로 요청 수를 카운팅해 임계값 초과 시 429를 반환한다.
// 카운터는 Proxy-Wasm shared data API에 저장한다 (ADR-0001).

/// config JSON `{"requests_per_second": N}` 에서 N을 파싱한다.
/// 파싱 실패 시 기본값 10을 반환한다.
pub fn parse_config(s: &str) -> u64 {
    // 단순 문자열 파싱 — 외부 JSON 크레이트 없이 구현
    // {"requests_per_second": 10} 형태만 처리
    let s = s.trim();
    if let Some(start) = s.find("\"requests_per_second\"") {
        let rest = &s[start + "\"requests_per_second\"".len()..];
        // ':' 이후 숫자 찾기
        if let Some(colon) = rest.find(':') {
            let after_colon = rest[colon + 1..].trim();
            // 숫자 부분만 추출
            let digits: String = after_colon
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(n) = digits.parse::<u64>() {
                return n;
            }
        }
    }
    10 // 기본값
}

/// 현재 카운터가 limit에 도달했으면 true (429 반환해야 함).
pub fn should_rate_limit(current: u64, limit: u64) -> bool {
    current >= limit
}

#[cfg(target_arch = "wasm32")]
mod filter {
    use super::{parse_config, should_rate_limit};
    use proxy_wasm::traits::{Context, HttpContext, RootContext};
    use proxy_wasm::types::{Action, ContextType, LogLevel};
    use std::cell::RefCell;
    use std::collections::HashMap;

    // wasm32는 단일 스레드이므로 thread_local이 안전하게 가변 상태를 보유한다.
    // per-worker 카운터 — ADR-0001 참고.
    thread_local! {
        static COUNTERS: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
        static LIMIT: RefCell<u64> = RefCell::new(10);
    }

    proxy_wasm::main! {{
        proxy_wasm::set_log_level(LogLevel::Info);
        proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
            Box::new(RateLimitRoot)
        });
    }}

    struct RateLimitRoot;
    impl Context for RateLimitRoot {}

    impl RootContext for RateLimitRoot {
        fn on_configure(&mut self, _: usize) -> bool {
            if let Some(bytes) = self.get_plugin_configuration() {
                if let Ok(s) = std::str::from_utf8(&bytes) {
                    let limit = parse_config(s);
                    LIMIT.with(|l| *l.borrow_mut() = limit);
                }
            }
            true
        }

        fn get_type(&self) -> Option<ContextType> {
            Some(ContextType::HttpContext)
        }

        fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
            Some(Box::new(RateLimitFilter))
        }
    }

    struct RateLimitFilter;
    impl Context for RateLimitFilter {}

    impl HttpContext for RateLimitFilter {
        fn on_http_request_headers(&mut self, _: usize, _: bool) -> Action {
            let api_key = self
                .get_http_request_header("x-api-key")
                .unwrap_or_else(|| "anonymous".to_string());

            let limit = LIMIT.with(|l| *l.borrow());
            let new_count = COUNTERS.with(|c| {
                let mut map = c.borrow_mut();
                let count = map.entry(api_key).or_insert(0);
                *count += 1;
                *count
            });

            if should_rate_limit(new_count, limit) {
                self.send_http_response(
                    429,
                    vec![("content-type", "text/plain")],
                    Some(b"Too Many Requests"),
                );
                return Action::Pause;
            }

            Action::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_config, should_rate_limit};

    // --- parse_config tests ---

    #[test]
    fn parse_config_returns_value_from_json() {
        assert_eq!(parse_config(r#"{"requests_per_second": 10}"#), 10);
    }

    #[test]
    fn parse_config_handles_large_limit() {
        assert_eq!(parse_config(r#"{"requests_per_second": 1000}"#), 1000);
    }

    #[test]
    fn parse_config_returns_default_on_empty_input() {
        assert_eq!(parse_config(""), 10);
    }

    #[test]
    fn parse_config_returns_default_on_missing_key() {
        assert_eq!(parse_config(r#"{"other_key": 5}"#), 10);
    }

    #[test]
    fn parse_config_handles_whitespace() {
        assert_eq!(parse_config(r#"{ "requests_per_second" : 42 }"#), 42);
    }

    // --- should_rate_limit tests ---

    #[test]
    fn should_rate_limit_false_when_below_limit() {
        assert!(!should_rate_limit(5, 10));
    }

    #[test]
    fn should_rate_limit_true_when_at_limit() {
        assert!(should_rate_limit(10, 10));
    }

    #[test]
    fn should_rate_limit_true_when_above_limit() {
        assert!(should_rate_limit(11, 10));
    }

    #[test]
    fn should_rate_limit_false_when_zero_count() {
        assert!(!should_rate_limit(0, 10));
    }

    #[test]
    fn should_rate_limit_true_when_limit_is_one_and_count_is_one() {
        assert!(should_rate_limit(1, 1));
    }
}
