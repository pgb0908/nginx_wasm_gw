use std::{fs, path::Path, process::Command};

pub enum NginxStatus {
    Running(u32),
    Stopped,
    StalePid(u32),
}

/// ngx_wasm_module resolves wasm module paths relative to the process cwd, not the prefix.
/// Run nginx with cwd=nginx_prefix and -p . so relative paths in nginx.conf work correctly.
fn nginx_cmd(nginx_bin: &Path, nginx_prefix: &Path, args: &[&str]) -> Result<std::process::ExitStatus, String> {
    let abs_bin = std::fs::canonicalize(nginx_bin)
        .map_err(|e| format!("nginx binary {nginx_bin:?}: {e}"))?;
    let abs_prefix = std::fs::canonicalize(nginx_prefix)
        .map_err(|e| format!("nginx prefix {nginx_prefix:?}: {e}"))?;
    Command::new(&abs_bin)
        .arg("-p").arg(".")
        .arg("-c").arg("nginx.conf")
        .args(args)
        .current_dir(&abs_prefix)
        .status()
        .map_err(|e| format!("failed to run nginx: {e}"))
}

pub fn start(nginx_bin: &Path, nginx_prefix: &Path) -> Result<(), String> {
    let st = nginx_cmd(nginx_bin, nginx_prefix, &[])?;
    if st.success() { Ok(()) } else { Err(format!("nginx exited with {st}")) }
}

pub fn stop(nginx_bin: &Path, nginx_prefix: &Path) -> Result<(), String> {
    let st = nginx_cmd(nginx_bin, nginx_prefix, &["-s", "stop"])?;
    if st.success() { Ok(()) } else { Err(format!("nginx -s stop failed with {st}")) }
}

pub fn reload(nginx_bin: &Path, nginx_prefix: &Path) -> Result<(), String> {
    let st = nginx_cmd(nginx_bin, nginx_prefix, &["-s", "reload"])?;
    if st.success() { Ok(()) } else { Err(format!("nginx -s reload failed with {st}")) }
}

pub fn status(nginx_prefix: &Path) -> NginxStatus {
    let pid_path = nginx_prefix.join("logs/nginx.pid");
    let content = match fs::read_to_string(&pid_path) {
        Ok(c) => c,
        Err(_) => return NginxStatus::Stopped,
    };
    let pid: u32 = match content.trim().parse() {
        Ok(p) => p,
        Err(_) => return NginxStatus::Stopped,
    };
    if process_exists(pid) {
        NginxStatus::Running(pid)
    } else {
        NginxStatus::StalePid(pid)
    }
}

fn process_exists(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_prefix() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        dir
    }

    // Cycle 1 (tracer): PID 파일 없음 → Stopped
    #[test]
    fn status_stopped_when_no_pid_file() {
        let dir = make_prefix();
        assert!(matches!(status(dir.path()), NginxStatus::Stopped));
    }

    // Cycle 2: 현재 프로세스 PID → Running
    #[test]
    fn status_running_when_pid_is_current_process() {
        let dir = make_prefix();
        let pid = std::process::id();
        fs::write(dir.path().join("logs/nginx.pid"), pid.to_string()).unwrap();
        assert!(matches!(status(dir.path()), NginxStatus::Running(p) if p == pid));
    }

    // Cycle 3: 존재하지 않는 PID → StalePid
    #[test]
    fn status_stale_when_pid_not_running() {
        let dir = make_prefix();
        fs::write(dir.path().join("logs/nginx.pid"), "2147483647").unwrap();
        assert!(matches!(status(dir.path()), NginxStatus::StalePid(2147483647)));
    }
}
