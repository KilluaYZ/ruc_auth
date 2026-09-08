//! ruc-auth — 人大校园网 (go.ruc.edu.cn, 深澜 srun_bx1) 认证登录 CLI
mod config;
mod crypto;
mod portal;
mod serve;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ruc-auth",
    version,
    about = "人大校园网 (go.ruc.edu.cn) 认证登录工具"
)]
struct Cli {
    /// 配置文件路径 (默认 ~/.config/ruc-auth/config.yaml)
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 执行一次登录
    Login,
    /// 常驻运行, 周期检测网络, 掉线自动重新登录
    Serve {
        /// 检测间隔秒数 (默认取配置 interval)
        #[arg(short = 'i', long)]
        interval: Option<u64>,
        /// 转入后台以守护进程方式运行
        #[arg(short = 'd', long)]
        detach: bool,
    },
    /// 读取 / 写入配置 (git config 风格)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 初始化配置文件
    Init,
    /// 读取配置项 (不带参数时列出全部)
    Get {
        /// 配置项名称, 如 username / password / ac_id / timeout ...
        key: Option<String>,
    },
    /// 写入配置项
    Set {
        /// 配置项名称
        key: String,
        /// 配置项值
        value: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let config_path = cli.config.clone().unwrap_or_else(config::default_path);

    let result = match cli.command {
        Command::Login => portal::login_cmd(&config_path),
        Command::Serve { interval, detach } => serve::serve_cmd(&config_path, interval, detach),
        Command::Config { action } => config::config_cmd(&config_path, &action),
    };
    if let Err(e) = result {
        eprintln!("[x] {}", e);
        std::process::exit(1);
    }
}
