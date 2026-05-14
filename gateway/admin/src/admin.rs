use std::path::Path;

use crate::{config::GatewayConfig, nginx, template};

/// Load Resource Model from `config_dir` and render to nginx.conf string.
pub fn generate_conf(config_dir: &Path, wasm_dir: &str) -> Result<String, String> {
    let cfg = GatewayConfig::load(config_dir)?;
    Ok(template::render(&cfg, wasm_dir))
}

/// generate_conf → write nginx.conf → nginx -s reload.
/// Returns Err (and skips reload) if config parsing or file write fails.
pub fn try_reload(
    config_dir: &Path,
    nginx_conf_path: &Path,
    nginx_bin: &Path,
    nginx_prefix: &Path,
    wasm_dir: &str,
) -> Result<(), String> {
    let conf = generate_conf(config_dir, wasm_dir)?;
    std::fs::write(nginx_conf_path, &conf).map_err(|e| format!("write nginx.conf: {e}"))?;
    nginx::reload(nginx_bin, nginx_prefix)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const DEFAULT_WASM_DIR: &str = "../../target/wasm32-wasip1/wasm-release";

    fn valid_config_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        for sub in &["listeners", "routers", "services", "policies"] {
            fs::create_dir_all(dir.path().join(sub)).unwrap();
        }
        fs::write(
            dir.path().join("listeners/l.json"),
            r#"{"apiVersion":"iip.gateway/v1alpha1","kind":"Listener",
               "metadata":{"name":"l"},"spec":{"protocol":"HTTP","port":9000}}"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn generate_conf_fails_for_invalid_json() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("listeners")).unwrap();
        fs::write(dir.path().join("listeners/bad.json"), b"not json").unwrap();
        assert!(generate_conf(dir.path(), DEFAULT_WASM_DIR).is_err());
    }

    #[test]
    fn generate_conf_returns_conf_with_listen_port() {
        let dir = valid_config_dir();
        let conf = generate_conf(dir.path(), DEFAULT_WASM_DIR).unwrap();
        assert!(conf.contains("worker_processes"), "missing preamble: {conf}");
        assert!(conf.contains("listen 9000"), "missing listen port: {conf}");
    }

    #[test]
    fn generate_conf_uses_custom_wasm_dir() {
        let dir = valid_config_dir();
        let conf = generate_conf(dir.path(), "../filters").unwrap();
        assert!(conf.contains("../filters/api_key.wasm"), "got: {conf}");
    }
}
