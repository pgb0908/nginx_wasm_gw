use crate::config::{GatewayConfig, HeaderControlConfig, Listener, Policy, PolicyConfig, Router, Service};

const STATIC_PREAMBLE: &str = r#"worker_processes auto;
error_log logs/error.log info;
pid logs/nginx.pid;

env RUST_BACKTRACE=full;

events {
    worker_connections 1024;
}"#;

const HTTP_SETTINGS: &str = "    default_type application/octet-stream;\n    client_body_temp_path tmp/client_body;\n    proxy_temp_path       tmp/proxy;\n    fastcgi_temp_path     tmp/fastcgi;\n    scgi_temp_path        tmp/scgi;\n    uwsgi_temp_path       tmp/uwsgi;\n\n    proxy_http_version 1.1;\n    proxy_set_header   Connection \"\";";

fn render_wasm_block(wasm_dir: &str) -> String {
    format!(
        "wasm {{\n\
        \x20   module rate_limit_filter          {wasm_dir}/rate_limit.wasm;\n\
        \x20   module api_key_filter             {wasm_dir}/api_key.wasm;\n\
        \x20   module header_manipulation_filter {wasm_dir}/header_manipulation.wasm;\n\
        \x20   module logging_filter             {wasm_dir}/logging.wasm;\n\
        \x20   module passthrough_filter         {wasm_dir}/passthrough.wasm;\n\
        }}"
    )
}

pub fn render(cfg: &GatewayConfig, wasm_dir: &str) -> String {
    let mut out = String::new();
    out.push_str(STATIC_PREAMBLE);
    out.push_str("\n\n");
    out.push_str(&render_wasm_block(wasm_dir));
    out.push_str("\n\nhttp {\n");
    out.push_str(HTTP_SETTINGS);
    out.push('\n');

    for svc in &cfg.services {
        out.push('\n');
        out.push_str(&render_upstream(svc));
    }

    for listener in &cfg.listeners {
        out.push('\n');
        out.push_str(&render_server(listener, cfg));
    }

    out.push_str("}\n");
    out
}

fn render_upstream(svc: &Service) -> String {
    let mut out = String::new();
    out.push_str(&format!("    upstream {} {{\n", svc.metadata.name));
    for t in &svc.spec.load_balancing.targets {
        out.push_str(&format!("        server {}:{} weight={};\n", t.host, t.port, t.weight));
    }
    out.push_str("        keepalive 32;\n");
    out.push_str("    }\n");
    out
}

fn render_server(listener: &Listener, cfg: &GatewayConfig) -> String {
    let mut out = String::new();
    out.push_str("    server {\n");
    out.push_str(&format!("        listen {};\n", listener.spec.port));

    let routers: Vec<&Router> = cfg
        .routers
        .iter()
        .filter(|r| r.spec.target_ref.name == listener.metadata.name)
        .collect();

    for router in routers {
        for rule in &router.spec.rules {
            out.push('\n');
            out.push_str(&render_location(rule.path.as_str(), router, cfg));
        }
    }

    out.push_str("    }\n");
    out
}

fn render_location(path: &str, router: &Router, cfg: &GatewayConfig) -> String {
    let mut out = String::new();
    out.push_str(&format!("        location {} {{\n", path));

    let applicable: Vec<&Policy> = cfg
        .policies
        .iter()
        .filter(|p| {
            let n = &p.spec.target_ref.name;
            n == "*" || n == &router.metadata.name
        })
        .collect();

    for policy in &applicable {
        if let Some(directive) = render_proxy_wasm(policy) {
            out.push_str(&format!("            {}\n", directive));
        }
    }
    out.push_str("            proxy_wasm logging_filter;\n");

    let service_name = router
        .spec
        .config
        .destinations
        .first()
        .map(|d| d.destination_ref.name.as_str())
        .unwrap_or("");
    out.push_str(&format!("            proxy_pass http://{};\n", service_name));

    out.push_str("        }\n");
    out
}

fn render_proxy_wasm(policy: &Policy) -> Option<String> {
    match &policy.spec.config {
        PolicyConfig::Security(s) => {
            let ak = s.api_key.as_ref()?;
            let keys: Vec<String> = ak.keys.iter().map(|k| format!("\"{}\"", k)).collect();
            let json = format!("{{\"api_keys\":[{}]}}", keys.join(","));
            Some(format!("proxy_wasm api_key_filter '{}';", json))
        }
        PolicyConfig::Traffic(t) => {
            let rl = t.rate_limit.as_ref()?;
            let json = format!("{{\"requests_per_second\":{}}}", rl.quota.limit);
            Some(format!("proxy_wasm rate_limit_filter '{}';", json))
        }
        PolicyConfig::Transform(t) => {
            let hc = t.header_control.as_ref()?;
            let json = render_header_control_json(hc);
            Some(format!("proxy_wasm header_manipulation_filter '{}';", json))
        }
        PolicyConfig::Enhance(_) => None,
    }
}

fn render_header_control_json(hc: &HeaderControlConfig) -> String {
    use serde_json::{Map, Value};

    let mut obj = Map::new();

    if let Some(req) = &hc.request {
        let mut req_obj = Map::new();
        if !req.add.is_empty() {
            req_obj.insert("add".to_string(), serde_json::to_value(&req.add).unwrap_or(Value::Null));
        }
        if !req.remove.is_empty() {
            req_obj.insert("remove".to_string(), serde_json::to_value(&req.remove).unwrap_or(Value::Null));
        }
        obj.insert("request".to_string(), Value::Object(req_obj));
    }

    if let Some(res) = &hc.response {
        let mut res_obj = Map::new();
        if !res.add.is_empty() {
            res_obj.insert("add".to_string(), serde_json::to_value(&res.add).unwrap_or(Value::Null));
        }
        if !res.remove.is_empty() {
            res_obj.insert("remove".to_string(), serde_json::to_value(&res.remove).unwrap_or(Value::Null));
        }
        obj.insert("response".to_string(), Value::Object(res_obj));
    }

    serde_json::to_string(&Value::Object(obj)).unwrap_or_default()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ApiKeyConfig, Destination, HeaderControlConfig, HeaderDirectionConfig,
        LoadBalancing, Metadata, PolicyConfig, PolicySpec, RateLimitConfig,
        RateLimitQuota, ResourceRef, RouterConfig, RouterRule, RouterSpec, SecurityConfig,
        ServiceSpec, Target, TrafficConfig, TransformConfig,
    };
    use std::collections::HashMap;

    fn meta(name: &str) -> Metadata {
        Metadata { name: name.to_string(), labels: Default::default() }
    }

    fn make_service(name: &str, host: &str, port: u16) -> Service {
        Service {
            metadata: meta(name),
            spec: ServiceSpec {
                protocol: "HTTP".to_string(),
                load_balancing: LoadBalancing {
                    algorithm: "ROUND_ROBIN".to_string(),
                    targets: vec![Target { host: host.to_string(), port, weight: 1 }],
                },
            },
        }
    }

    fn make_router(name: &str, path: &str, listener: &str, service: &str) -> Router {
        Router {
            metadata: meta(name),
            spec: RouterSpec {
                target_ref: ResourceRef { kind: Some("Listener".to_string()), name: listener.to_string() },
                rules: vec![RouterRule { path: path.to_string(), methods: None }],
                config: RouterConfig {
                    destinations: vec![Destination {
                        destination_ref: ResourceRef { kind: Some("Service".to_string()), name: service.to_string() },
                        weight: 100,
                        rewrite: None,
                    }],
                },
            },
        }
    }

    fn make_listener(name: &str, port: u16) -> Listener {
        Listener {
            metadata: meta(name),
            spec: crate::config::ListenerSpec {
                protocol: "HTTP".to_string(),
                port,
                host: "0.0.0.0".to_string(),
            },
        }
    }

    fn make_policy(name: &str, order: u32, config: PolicyConfig) -> Policy {
        Policy {
            metadata: meta(name),
            spec: PolicySpec {
                target_ref: ResourceRef { kind: None, name: "*".to_string() },
                order,
                rules: None,
                config,
            },
        }
    }

    // ── Cycle 1: upstream block ──────────────────────────────────────────────

    #[test]
    fn renders_upstream_block() {
        let svc = make_service("svc-v1", "127.0.0.1", 8081);
        let out = render_upstream(&svc);
        assert!(out.contains("upstream svc-v1 {"), "got: {out}");
        assert!(out.contains("server 127.0.0.1:8081"), "got: {out}");
        assert!(out.contains("keepalive 32"), "got: {out}");
    }

    // ── Cycle 2: server block ────────────────────────────────────────────────

    #[test]
    fn renders_server_listen_port() {
        let listener = make_listener("default-listener", 9000);
        let cfg = GatewayConfig { listeners: vec![], routers: vec![], services: vec![], policies: vec![] };
        let out = render_server(&listener, &cfg);
        assert!(out.contains("listen 9000;"), "got: {out}");
    }

    // ── Cycle 3: location block with proxy_pass ──────────────────────────────

    #[test]
    fn renders_location_with_proxy_pass() {
        let router = make_router("r", "/api/v1/", "l", "svc-v1");
        let cfg = GatewayConfig { listeners: vec![], routers: vec![], services: vec![], policies: vec![] };
        let out = render_location("/api/v1/", &router, &cfg);
        assert!(out.contains("location /api/v1/ {"), "got: {out}");
        assert!(out.contains("proxy_pass http://svc-v1;"), "got: {out}");
    }

    // ── Cycle 4: Security proxy_wasm ─────────────────────────────────────────

    #[test]
    fn renders_api_key_filter_directive() {
        let policy = make_policy("sec", 5, PolicyConfig::Security(SecurityConfig {
            api_key: Some(ApiKeyConfig {
                header: "X-API-Key".to_string(),
                keys: vec!["k1".to_string(), "k2".to_string()],
            }),
            ..Default::default()
        }));
        let out = render_proxy_wasm(&policy).unwrap();
        assert!(out.contains("proxy_wasm api_key_filter"), "got: {out}");
        assert!(out.contains("\"k1\""), "got: {out}");
        assert!(out.contains("\"k2\""), "got: {out}");
    }

    // ── Cycle 5: Traffic proxy_wasm ──────────────────────────────────────────

    #[test]
    fn renders_rate_limit_filter_directive() {
        let policy = make_policy("traffic", 10, PolicyConfig::Traffic(TrafficConfig {
            rate_limit: Some(RateLimitConfig {
                quota: RateLimitQuota { limit: 20, window: "1s".to_string() },
                burst: None,
            }),
            ..Default::default()
        }));
        let out = render_proxy_wasm(&policy).unwrap();
        assert!(out.contains("proxy_wasm rate_limit_filter"), "got: {out}");
        assert!(out.contains("\"requests_per_second\":20"), "got: {out}");
    }

    // ── Cycle 6: Transform proxy_wasm ────────────────────────────────────────

    #[test]
    fn renders_header_manipulation_filter_directive() {
        let mut add = HashMap::new();
        add.insert("X-Gateway".to_string(), "gw".to_string());
        let policy = make_policy("transform", 15, PolicyConfig::Transform(TransformConfig {
            header_control: Some(HeaderControlConfig {
                request: Some(HeaderDirectionConfig {
                    add: add.clone(),
                    remove: vec!["X-Secret".to_string()],
                }),
                response: None,
            }),
            ..Default::default()
        }));
        let out = render_proxy_wasm(&policy).unwrap();
        assert!(out.contains("proxy_wasm header_manipulation_filter"), "got: {out}");
        assert!(out.contains("X-Gateway"), "got: {out}");
        assert!(out.contains("X-Secret"), "got: {out}");
    }

    // ── Cycle 7: logging_filter always last ──────────────────────────────────

    #[test]
    fn logging_filter_is_last_proxy_wasm() {
        let router = make_router("r", "/api/", "l", "svc");
        let cfg = GatewayConfig {
            listeners: vec![],
            routers: vec![],
            services: vec![],
            policies: vec![make_policy("sec", 5, PolicyConfig::Security(SecurityConfig {
                api_key: Some(ApiKeyConfig {
                    header: "X-API-Key".to_string(),
                    keys: vec!["k".to_string()],
                }),
                ..Default::default()
            }))],
        };
        let out = render_location("/api/", &router, &cfg);
        let api_key_pos = out.find("proxy_wasm api_key_filter").unwrap();
        let logging_pos = out.find("proxy_wasm logging_filter").unwrap();
        assert!(logging_pos > api_key_pos, "logging_filter must come after api_key_filter");
        assert!(out.ends_with("        }\n"), "location block must close");
    }

    // ── Cycle 8: policy order preserved ─────────────────────────────────────

    #[test]
    fn policies_rendered_in_order() {
        let router = make_router("r", "/api/", "l", "svc");
        let cfg = GatewayConfig {
            listeners: vec![],
            routers: vec![],
            services: vec![],
            policies: vec![
                make_policy("sec", 5, PolicyConfig::Security(SecurityConfig {
                    api_key: Some(ApiKeyConfig {
                        header: "X-API-Key".to_string(),
                        keys: vec!["k".to_string()],
                    }),
                    ..Default::default()
                })),
                make_policy("traffic", 10, PolicyConfig::Traffic(TrafficConfig {
                    rate_limit: Some(RateLimitConfig {
                        quota: RateLimitQuota { limit: 10, window: "1s".to_string() },
                        burst: None,
                    }),
                    ..Default::default()
                })),
            ],
        };
        let out = render_location("/api/", &router, &cfg);
        let api_key_pos = out.find("api_key_filter").unwrap();
        let rate_limit_pos = out.find("rate_limit_filter").unwrap();
        assert!(api_key_pos < rate_limit_pos, "api_key (order 5) must precede rate_limit (order 10)");
    }

    // ── Cycle 9: full render structure ───────────────────────────────────────

    #[test]
    fn full_render_contains_all_sections() {
        let cfg = GatewayConfig {
            listeners: vec![make_listener("default-listener", 9000)],
            routers: vec![make_router("api-v1", "/api/v1/", "default-listener", "svc-v1")],
            services: vec![make_service("svc-v1", "127.0.0.1", 8081)],
            policies: vec![],
        };
        let out = render(&cfg, "../../target/wasm32-wasip1/wasm-release");
        assert!(out.contains("worker_processes auto;"), "missing static header");
        assert!(out.contains("upstream svc-v1 {"), "missing upstream");
        assert!(out.contains("listen 9000;"), "missing listen");
        assert!(out.contains("location /api/v1/ {"), "missing location");
        assert!(out.contains("proxy_pass http://svc-v1;"), "missing proxy_pass");
        assert!(out.contains("proxy_wasm logging_filter;"), "missing logging_filter");
    }

    // ── Cycle 10: custom wasm_dir ────────────────────────────────────────────

    #[test]
    fn render_uses_custom_wasm_dir() {
        let cfg = GatewayConfig {
            listeners: vec![make_listener("l", 9000)],
            routers: vec![],
            services: vec![],
            policies: vec![],
        };
        let out = render(&cfg, "../filters");
        assert!(out.contains("../filters/api_key.wasm"), "got: {out}");
        assert!(out.contains("../filters/rate_limit.wasm"), "got: {out}");
        assert!(out.contains("../filters/logging.wasm"), "got: {out}");
        assert!(!out.contains("wasm-release"), "default path must not appear: {out}");
    }
}
