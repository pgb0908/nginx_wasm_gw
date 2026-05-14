use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ── Shared ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    pub name: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceRef {
    pub kind: Option<String>,
    pub name: String,
}

// ── Listener ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Listener {
    pub metadata: Metadata,
    pub spec: ListenerSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenerSpec {
    pub protocol: String,
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

// ── Service ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Service {
    pub metadata: Metadata,
    pub spec: ServiceSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceSpec {
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(rename = "loadBalancing")]
    pub load_balancing: LoadBalancing,
}

fn default_protocol() -> String {
    "HTTP".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoadBalancing {
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    pub targets: Vec<Target>,
}

fn default_algorithm() -> String {
    "ROUND_ROBIN".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_weight_one")]
    pub weight: u32,
}

fn default_weight_one() -> u32 {
    1
}

// ── Router ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Router {
    pub metadata: Metadata,
    pub spec: RouterSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterSpec {
    #[serde(rename = "targetRef")]
    pub target_ref: ResourceRef,
    pub rules: Vec<RouterRule>,
    pub config: RouterConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterRule {
    pub path: String,
    pub methods: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouterConfig {
    pub destinations: Vec<Destination>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Destination {
    #[serde(rename = "destinationRef")]
    pub destination_ref: ResourceRef,
    #[serde(default = "default_weight_hundred")]
    pub weight: u32,
    pub rewrite: Option<Rewrite>,
}

fn default_weight_hundred() -> u32 {
    100
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rewrite {
    pub path: Option<String>,
    pub host: Option<String>,
}

// ── Policy ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Policy {
    pub metadata: Metadata,
    pub spec: PolicySpec,
}

#[derive(Debug, Clone)]
pub struct PolicySpec {
    pub target_ref: ResourceRef,
    pub order: u32,
    pub rules: Option<Vec<PolicyRule>>,
    pub config: PolicyConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyRule {
    pub path: Option<String>,
    pub methods: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PolicyConfig {
    Security(SecurityConfig),
    Traffic(TrafficConfig),
    Transform(TransformConfig),
    Enhance(EnhanceConfig),
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(rename = "apiKey")]
    pub api_key: Option<ApiKeyConfig>,
    #[serde(rename = "jwtValidation")]
    pub jwt_validation: Option<Value>,
    #[serde(rename = "ipFilter")]
    pub ip_filter: Option<Value>,
    pub cors: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyConfig {
    #[serde(default = "default_api_key_header")]
    pub header: String,
    pub keys: Vec<String>,
}

fn default_api_key_header() -> String {
    "X-API-Key".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TrafficConfig {
    #[serde(rename = "rateLimit")]
    pub rate_limit: Option<RateLimitConfig>,
    #[serde(rename = "maxConcurrency")]
    pub max_concurrency: Option<Value>,
    #[serde(rename = "slaTiers")]
    pub sla_tiers: Option<Vec<Value>>,
    pub strategy: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub quota: RateLimitQuota,
    pub burst: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitQuota {
    pub limit: u64,
    pub window: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TransformConfig {
    #[serde(rename = "headerControl")]
    pub header_control: Option<HeaderControlConfig>,
    #[serde(rename = "queryControl")]
    pub query_control: Option<Value>,
    #[serde(rename = "bodyTransformation")]
    pub body_transformation: Option<Value>,
    #[serde(rename = "dataMasking")]
    pub data_masking: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeaderControlConfig {
    pub request: Option<HeaderDirectionConfig>,
    pub response: Option<HeaderDirectionConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeaderDirectionConfig {
    #[serde(default)]
    pub add: HashMap<String, String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnhanceConfig {
    pub ttl: Option<u64>,
    pub strategy: Option<String>,
    #[serde(rename = "storageRef")]
    pub storage_ref: Option<Value>,
}

// ── Policy parsing ────────────────────────────────────────────────────────────

fn parse_policy_config(v: Value) -> Result<PolicyConfig, String> {
    let obj = v.as_object().ok_or("policy config must be an object")?;

    let is_security = ["apiKey", "jwtValidation", "ipFilter", "cors"]
        .iter()
        .any(|k| obj.contains_key(*k));
    let is_traffic = ["rateLimit", "maxConcurrency", "slaTiers"]
        .iter()
        .any(|k| obj.contains_key(*k));
    let is_transform = ["headerControl", "queryControl", "bodyTransformation", "dataMasking"]
        .iter()
        .any(|k| obj.contains_key(*k));
    let is_enhance = obj.contains_key("ttl");

    if is_security {
        let cfg: SecurityConfig =
            serde_json::from_value(v).map_err(|e| format!("security config: {e}"))?;
        Ok(PolicyConfig::Security(cfg))
    } else if is_traffic {
        let cfg: TrafficConfig =
            serde_json::from_value(v).map_err(|e| format!("traffic config: {e}"))?;
        Ok(PolicyConfig::Traffic(cfg))
    } else if is_transform {
        let cfg: TransformConfig =
            serde_json::from_value(v).map_err(|e| format!("transform config: {e}"))?;
        Ok(PolicyConfig::Transform(cfg))
    } else if is_enhance {
        let cfg: EnhanceConfig =
            serde_json::from_value(v).map_err(|e| format!("enhance config: {e}"))?;
        Ok(PolicyConfig::Enhance(cfg))
    } else {
        Err(format!("unknown policy config keys: {:?}", obj.keys().collect::<Vec<_>>()))
    }
}

fn parse_policy(json: &str) -> Result<Policy, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;

    let metadata: Metadata = serde_json::from_value(v["metadata"].clone())
        .map_err(|e| format!("metadata: {e}"))?;
    let spec = &v["spec"];

    let target_ref: ResourceRef = serde_json::from_value(spec["targetRef"].clone())
        .map_err(|e| format!("targetRef: {e}"))?;
    let order = spec["order"].as_u64().unwrap_or(10) as u32;
    let rules: Option<Vec<PolicyRule>> = spec["rules"]
        .as_array()
        .map(|arr| serde_json::from_value(Value::Array(arr.clone())))
        .transpose()
        .map_err(|e: serde_json::Error| format!("rules: {e}"))?;
    let config = parse_policy_config(spec["config"].clone())?;

    Ok(Policy {
        metadata,
        spec: PolicySpec { target_ref, order, rules, config },
    })
}

// ── Gateway ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Gateway {
    pub spec: GatewaySpec,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GatewaySpec {
    #[serde(default)]
    pub logging: GatewayLogging,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GatewayLogging {
    #[serde(rename = "errorLog", default)]
    pub error_log: ErrorLogConfig,
    #[serde(rename = "accessLog", default)]
    pub access_log: AccessLogConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorLogConfig {
    pub level: String,
}

impl Default for ErrorLogConfig {
    fn default() -> Self {
        Self { level: "info".to_string() }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccessLogConfig {
    pub enabled: bool,
    pub format: String,
}

impl Default for AccessLogConfig {
    fn default() -> Self {
        Self { enabled: true, format: "JSON".to_string() }
    }
}

// ── GatewayConfig ─────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct GatewayConfig {
    pub gateway: Option<Gateway>,
    pub listeners: Vec<Listener>,
    pub routers: Vec<Router>,
    pub services: Vec<Service>,
    pub policies: Vec<Policy>,
}

impl GatewayConfig {
    pub fn load(config_dir: &Path) -> Result<Self, String> {
        let mut cfg = GatewayConfig::default();

        cfg.gateway = load_gateway(config_dir.join("gateways").as_path())?;
        cfg.listeners = load_resources(config_dir.join("listeners").as_path())?;
        cfg.routers = load_resources(config_dir.join("routers").as_path())?;
        cfg.services = load_resources(config_dir.join("services").as_path())?;
        cfg.policies = load_policies(config_dir.join("policies").as_path())?;

        cfg.policies.sort_by_key(|p| p.spec.order);

        Ok(cfg)
    }
}

fn load_gateway(dir: &Path) -> Result<Option<Gateway>, String> {
    if !dir.exists() {
        return Ok(None);
    }
    let files: Vec<_> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();

    match files.len() {
        0 => Ok(None),
        1 => {
            let content = fs::read_to_string(files[0].path()).map_err(|e| e.to_string())?;
            let gw: Gateway = serde_json::from_str(&content)
                .map_err(|e| format!("{}: {e}", files[0].path().display()))?;
            Ok(Some(gw))
        }
        n => Err(format!(
            "expected at most one Gateway file in {}, found {n}",
            dir.display()
        )),
    }
}

fn load_resources<T>(dir: &Path) -> Result<Vec<T>, String>
where
    T: for<'de> Deserialize<'de>,
{
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut resources = vec![];
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let resource: T = serde_json::from_str(&content)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        resources.push(resource);
    }
    Ok(resources)
}

fn load_policies(dir: &Path) -> Result<Vec<Policy>, String> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut policies = vec![];
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let policy = parse_policy(&content)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        policies.push(policy);
    }
    Ok(policies)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Cycle 1: Listener ────────────────────────────────────────────────────

    #[test]
    fn parses_listener_port_and_protocol() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Listener",
            "metadata": { "name": "default-listener" },
            "spec": { "protocol": "HTTP", "port": 9000, "host": "0.0.0.0" }
        }"#;
        let listener: Listener = serde_json::from_str(json).unwrap();
        assert_eq!(listener.metadata.name, "default-listener");
        assert_eq!(listener.spec.port, 9000);
        assert_eq!(listener.spec.protocol, "HTTP");
    }

    #[test]
    fn listener_host_defaults_to_0000() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Listener",
            "metadata": { "name": "l" },
            "spec": { "protocol": "HTTP", "port": 80 }
        }"#;
        let listener: Listener = serde_json::from_str(json).unwrap();
        assert_eq!(listener.spec.host, "0.0.0.0");
    }

    // ── Cycle 2: Service ─────────────────────────────────────────────────────

    #[test]
    fn parses_service_with_targets() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Service",
            "metadata": { "name": "svc-v1" },
            "spec": {
                "protocol": "HTTP",
                "loadBalancing": {
                    "algorithm": "ROUND_ROBIN",
                    "targets": [{ "host": "127.0.0.1", "port": 8081, "weight": 1 }]
                }
            }
        }"#;
        let svc: Service = serde_json::from_str(json).unwrap();
        assert_eq!(svc.metadata.name, "svc-v1");
        assert_eq!(svc.spec.load_balancing.targets[0].port, 8081);
        assert_eq!(svc.spec.load_balancing.targets[0].host, "127.0.0.1");
    }

    #[test]
    fn service_target_weight_defaults_to_one() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Service",
            "metadata": { "name": "s" },
            "spec": {
                "loadBalancing": {
                    "targets": [{ "host": "10.0.0.1", "port": 8080 }]
                }
            }
        }"#;
        let svc: Service = serde_json::from_str(json).unwrap();
        assert_eq!(svc.spec.load_balancing.targets[0].weight, 1);
    }

    // ── Cycle 3: Router ──────────────────────────────────────────────────────

    #[test]
    fn parses_router_rules_and_destination() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Router",
            "metadata": { "name": "api-v1" },
            "spec": {
                "targetRef": { "kind": "Listener", "name": "default-listener" },
                "rules": [{ "path": "/api/v1/" }],
                "config": {
                    "destinations": [{
                        "destinationRef": { "kind": "Service", "name": "svc-v1" },
                        "weight": 100
                    }]
                }
            }
        }"#;
        let router: Router = serde_json::from_str(json).unwrap();
        assert_eq!(router.metadata.name, "api-v1");
        assert_eq!(router.spec.rules[0].path, "/api/v1/");
        assert_eq!(router.spec.config.destinations[0].destination_ref.name, "svc-v1");
        assert_eq!(router.spec.target_ref.name, "default-listener");
    }

    // ── Cycle 4: Policy — Security (apiKey) ──────────────────────────────────

    #[test]
    fn parses_security_policy_with_api_key() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Policy",
            "metadata": { "name": "default-security" },
            "spec": {
                "targetRef": { "kind": "Router", "name": "*" },
                "order": 5,
                "config": {
                    "apiKey": {
                        "header": "X-API-Key",
                        "keys": ["secret-key-1", "secret-key-2"]
                    }
                }
            }
        }"#;
        let policy = parse_policy(json).unwrap();
        assert_eq!(policy.metadata.name, "default-security");
        assert_eq!(policy.spec.order, 5);
        match &policy.spec.config {
            PolicyConfig::Security(s) => {
                let ak = s.api_key.as_ref().unwrap();
                assert_eq!(ak.header, "X-API-Key");
                assert!(ak.keys.contains(&"secret-key-1".to_string()));
            }
            _ => panic!("expected Security config"),
        }
    }

    // ── Cycle 5: Policy — Traffic (rateLimit) ────────────────────────────────

    #[test]
    fn parses_traffic_policy_with_rate_limit() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Policy",
            "metadata": { "name": "default-traffic" },
            "spec": {
                "targetRef": { "kind": "Router", "name": "*" },
                "order": 10,
                "config": {
                    "rateLimit": {
                        "quota": { "limit": 10, "window": "1s" }
                    }
                }
            }
        }"#;
        let policy = parse_policy(json).unwrap();
        assert_eq!(policy.spec.order, 10);
        match &policy.spec.config {
            PolicyConfig::Traffic(t) => {
                let rl = t.rate_limit.as_ref().unwrap();
                assert_eq!(rl.quota.limit, 10);
                assert_eq!(rl.quota.window, "1s");
            }
            _ => panic!("expected Traffic config"),
        }
    }

    // ── Cycle 6: Policy — Transform (headerControl) ──────────────────────────

    #[test]
    fn parses_transform_policy_with_header_control() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Policy",
            "metadata": { "name": "default-transform" },
            "spec": {
                "targetRef": { "kind": "Router", "name": "*" },
                "order": 15,
                "config": {
                    "headerControl": {
                        "request": { "add": { "X-Gateway": "nginx-wasm-gw" }, "remove": ["X-Internal-Secret"] },
                        "response": { "add": { "X-Response-Time": "measured" }, "remove": ["Server"] }
                    }
                }
            }
        }"#;
        let policy = parse_policy(json).unwrap();
        assert_eq!(policy.spec.order, 15);
        match &policy.spec.config {
            PolicyConfig::Transform(t) => {
                let hc = t.header_control.as_ref().unwrap();
                let req = hc.request.as_ref().unwrap();
                assert_eq!(req.add.get("X-Gateway").map(|s| s.as_str()), Some("nginx-wasm-gw"));
                assert!(req.remove.contains(&"X-Internal-Secret".to_string()));
                let res = hc.response.as_ref().unwrap();
                assert!(res.remove.contains(&"Server".to_string()));
            }
            _ => panic!("expected Transform config"),
        }
    }

    // ── Cycle 7: policy order sort ───────────────────────────────────────────

    #[test]
    fn policies_are_sorted_by_order() {
        let make = |name: &str, order: u32| Policy {
            metadata: Metadata { name: name.to_string(), labels: Default::default() },
            spec: PolicySpec {
                target_ref: ResourceRef { kind: None, name: "*".to_string() },
                order,
                rules: None,
                config: PolicyConfig::Security(SecurityConfig::default()),
            },
        };
        let mut policies = vec![make("c", 15), make("a", 5), make("b", 10)];
        policies.sort_by_key(|p| p.spec.order);
        assert_eq!(policies[0].metadata.name, "a");
        assert_eq!(policies[1].metadata.name, "b");
        assert_eq!(policies[2].metadata.name, "c");
    }

    // ── Cycle 8: error handling ──────────────────────────────────────────────

    #[test]
    fn unknown_policy_config_returns_error() {
        let json = r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Policy",
            "metadata": { "name": "bad" },
            "spec": {
                "targetRef": { "name": "*" },
                "order": 5,
                "config": { "unknownField": true }
            }
        }"#;
        assert!(parse_policy(json).is_err());
    }
}

    // ── Cycle 9: Gateway 파싱 ────────────────────────────────────────────────

    #[test]
    fn gateway_is_none_when_directory_missing() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        // gateways/ 디렉토리 자체를 만들지 않음
        let result = load_gateway(dir.path().join("gateways").as_path());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn gateway_is_none_when_directory_empty() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("gateways")).unwrap();
        let result = load_gateway(dir.path().join("gateways").as_path());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn parses_gateway_logging_config() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let gw_dir = dir.path().join("gateways");
        std::fs::create_dir_all(&gw_dir).unwrap();
        std::fs::write(gw_dir.join("default.json"), r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Gateway",
            "metadata": { "name": "default-gateway" },
            "spec": {
                "logging": {
                    "errorLog": { "level": "warn" },
                    "accessLog": { "enabled": true, "format": "TEXT" }
                }
            }
        }"#).unwrap();
        let gw = load_gateway(&gw_dir).unwrap().unwrap();
        assert_eq!(gw.spec.logging.error_log.level, "warn");
        assert!(gw.spec.logging.access_log.enabled);
        assert_eq!(gw.spec.logging.access_log.format, "TEXT");
    }

    #[test]
    fn gateway_defaults_when_logging_omitted() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let gw_dir = dir.path().join("gateways");
        std::fs::create_dir_all(&gw_dir).unwrap();
        std::fs::write(gw_dir.join("default.json"), r#"{
            "apiVersion": "iip.gateway/v1alpha1",
            "kind": "Gateway",
            "metadata": { "name": "g" },
            "spec": {}
        }"#).unwrap();
        let gw = load_gateway(&gw_dir).unwrap().unwrap();
        assert_eq!(gw.spec.logging.error_log.level, "info");
        assert!(gw.spec.logging.access_log.enabled);
        assert_eq!(gw.spec.logging.access_log.format, "JSON");
    }

    #[test]
    fn two_gateway_files_returns_error() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let gw_dir = dir.path().join("gateways");
        std::fs::create_dir_all(&gw_dir).unwrap();
        std::fs::write(gw_dir.join("a.json"), r#"{"apiVersion":"iip.gateway/v1alpha1","kind":"Gateway","metadata":{"name":"a"},"spec":{}}"#).unwrap();
        std::fs::write(gw_dir.join("b.json"), r#"{"apiVersion":"iip.gateway/v1alpha1","kind":"Gateway","metadata":{"name":"b"},"spec":{}}"#).unwrap();
        assert!(load_gateway(&gw_dir).is_err());
    }

    #[test]
    fn loads_real_config_files() {
        let dir = std::path::Path::new("gateway/config");
        if !dir.exists() {
            return; // skip if run from different cwd
        }
        let cfg = GatewayConfig::load(dir).expect("should load without error");
        assert!(!cfg.listeners.is_empty(), "expected at least one listener");
        assert!(!cfg.routers.is_empty(), "expected at least one router");
        assert!(!cfg.services.is_empty(), "expected at least one service");
        assert!(!cfg.policies.is_empty(), "expected at least one policy");
        // policies sorted by order
        let orders: Vec<u32> = cfg.policies.iter().map(|p| p.spec.order).collect();
        assert!(orders.windows(2).all(|w| w[0] <= w[1]), "policies must be sorted by order");
    }
