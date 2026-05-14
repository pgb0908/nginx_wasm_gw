mod admin;
mod config;
mod nginx;
mod template;
mod watcher;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gw-admin", about = "IIP API Gateway admin process")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

const DEFAULT_WASM_DIR: &str = "../../target/wasm32-wasip1/wasm-release";

#[derive(Subcommand)]
enum Command {
    /// Start the gateway: generate nginx.conf, start nginx, watch config for changes
    Start {
        #[arg(long, default_value = "gateway/config")]
        config_dir: PathBuf,
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
        #[arg(long, default_value = "gateway/bin/nginx")]
        nginx_bin: PathBuf,
        #[arg(long, default_value = DEFAULT_WASM_DIR)]
        wasm_dir: String,
    },
    /// Stop nginx
    Stop {
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
        #[arg(long, default_value = "gateway/bin/nginx")]
        nginx_bin: PathBuf,
    },
    /// Reload nginx with regenerated config
    Reload {
        #[arg(long, default_value = "gateway/config")]
        config_dir: PathBuf,
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
        #[arg(long, default_value = "gateway/bin/nginx")]
        nginx_bin: PathBuf,
        #[arg(long, default_value = DEFAULT_WASM_DIR)]
        wasm_dir: String,
    },
    /// Show nginx status
    Status {
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
    },
    /// Generate nginx.conf from resource model (stdout or file)
    Generate {
        #[arg(long, default_value = "gateway/config")]
        config_dir: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = DEFAULT_WASM_DIR)]
        wasm_dir: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Start { config_dir, nginx_prefix, nginx_bin, wasm_dir } => {
            let nginx_conf_path = nginx_prefix.join("nginx.conf");
            let nginx_conf = admin::generate_conf(&config_dir, &wasm_dir).unwrap_or_else(|e| {
                eprintln!("error loading config: {e}");
                std::process::exit(1);
            });
            std::fs::write(&nginx_conf_path, &nginx_conf).unwrap_or_else(|e| {
                eprintln!("error writing nginx.conf: {e}");
                std::process::exit(1);
            });
            nginx::start(&nginx_bin, &nginx_prefix).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
            eprintln!("nginx started");
            watcher::start_watcher(config_dir, nginx_conf_path, nginx_bin, nginx_prefix, wasm_dir);
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        Command::Stop { nginx_prefix, nginx_bin } => {
            nginx::stop(&nginx_bin, &nginx_prefix).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
            eprintln!("nginx stopped");
        }
        Command::Reload { config_dir, nginx_prefix, nginx_bin, wasm_dir } => {
            let nginx_conf_path = nginx_prefix.join("nginx.conf");
            admin::try_reload(&config_dir, &nginx_conf_path, &nginx_bin, &nginx_prefix, &wasm_dir)
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                });
            eprintln!("nginx reloaded");
        }
        Command::Status { nginx_prefix } => {
            match nginx::status(&nginx_prefix) {
                nginx::NginxStatus::Running(pid) => println!("nginx is running (pid: {pid})"),
                nginx::NginxStatus::Stopped => println!("nginx is stopped"),
                nginx::NginxStatus::StalePid(pid) => {
                    println!("nginx is stopped (stale pid: {pid})");
                }
            }
        }
        Command::Generate { config_dir, out, wasm_dir } => {
            let nginx_conf = admin::generate_conf(&config_dir, &wasm_dir).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
            match out {
                Some(path) => {
                    std::fs::write(&path, &nginx_conf).unwrap_or_else(|e| {
                        eprintln!("error writing {}: {e}", path.display());
                        std::process::exit(1);
                    });
                }
                None => print!("{}", nginx_conf),
            }
        }
    }
}
