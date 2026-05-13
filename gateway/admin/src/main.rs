mod config;
mod template;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gw-admin", about = "IIP API Gateway admin process")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the gateway (nginx + config watcher)
    Start {
        #[arg(long, default_value = "gateway/config")]
        config_dir: PathBuf,
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
    },
    /// Stop the gateway
    Stop {
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
    },
    /// Reload nginx with updated config
    Reload {
        #[arg(long, default_value = "gateway/config")]
        config_dir: PathBuf,
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
    },
    /// Show nginx status
    Status {
        #[arg(long, default_value = "gateway/nginx")]
        nginx_prefix: PathBuf,
    },
    /// Generate nginx.conf from resource model
    Generate {
        #[arg(long, default_value = "gateway/config")]
        config_dir: PathBuf,
        /// Write output to file instead of stdout
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Start { config_dir, nginx_prefix } => {
            eprintln!("start: config_dir={config_dir:?} nginx_prefix={nginx_prefix:?}");
            eprintln!("not yet implemented");
        }
        Command::Stop { nginx_prefix } => {
            eprintln!("stop: nginx_prefix={nginx_prefix:?}");
            eprintln!("not yet implemented");
        }
        Command::Reload { config_dir, nginx_prefix } => {
            eprintln!("reload: config_dir={config_dir:?} nginx_prefix={nginx_prefix:?}");
            eprintln!("not yet implemented");
        }
        Command::Status { nginx_prefix } => {
            eprintln!("status: nginx_prefix={nginx_prefix:?}");
            eprintln!("not yet implemented");
        }
        Command::Generate { config_dir, out } => {
            let cfg = config::GatewayConfig::load(&config_dir).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
            let nginx_conf = template::render(&cfg);
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
