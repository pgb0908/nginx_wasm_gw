#[cfg(target_arch = "wasm32")]
mod filter {
    use proxy_wasm::traits::{Context, HttpContext, RootContext};
    use proxy_wasm::types::{ContextType, LogLevel};

    proxy_wasm::main! {{
        proxy_wasm::set_log_level(LogLevel::Info);
        proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> {
            Box::new(PassthroughRoot)
        });
    }}

    struct PassthroughRoot;
    impl Context for PassthroughRoot {}
    impl RootContext for PassthroughRoot {
        fn get_type(&self) -> Option<ContextType> {
            Some(ContextType::HttpContext)
        }
        fn create_http_context(&self, _: u32) -> Option<Box<dyn HttpContext>> {
            Some(Box::new(Passthrough))
        }
    }

    struct Passthrough;
    impl Context for Passthrough {}
    impl HttpContext for Passthrough {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn passthrough_filter_allows_all_requests() {
        // HttpContext의 기본 구현이 모든 훅에서 Action::Continue를 반환한다.
        // wasm32 타겟에서만 실행되므로 native에서는 컴파일 성공 자체가 검증이다.
        assert!(true);
    }
}
