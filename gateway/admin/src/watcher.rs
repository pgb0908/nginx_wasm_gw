use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

use crate::admin;

/// Spawn a background thread that watches `config_dir` recursively.
/// On any change: debounce → admin::try_reload. Errors are logged, nginx keeps running.
pub fn start_watcher(
    config_dir: PathBuf,
    nginx_conf_path: PathBuf,
    nginx_bin: PathBuf,
    nginx_prefix: PathBuf,
    wasm_dir: String,
) {
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<NotifyResult<Event>>();
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[watcher] failed to create watcher: {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&config_dir, RecursiveMode::Recursive) {
            eprintln!("[watcher] failed to watch {config_dir:?}: {e}");
            return;
        }
        eprintln!("[watcher] watching {config_dir:?}");

        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Ok(_event)) => {
                    // Debounce: drain all pending events before reloading
                    std::thread::sleep(Duration::from_millis(200));
                    while rx.try_recv().is_ok() {}

                    eprintln!("[watcher] config changed, reloading...");
                    match admin::try_reload(&config_dir, &nginx_conf_path, &nginx_bin, &nginx_prefix, &wasm_dir) {
                        Ok(()) => eprintln!("[watcher] reload ok"),
                        Err(e) => eprintln!("[watcher] reload skipped: {e}"),
                    }
                }
                Ok(Err(e)) => eprintln!("[watcher] watch error: {e}"),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

