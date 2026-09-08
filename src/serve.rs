//! serve 模式: 常驻运行, 周期性检测联网状态, 掉线自动重新登录
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::config::{self, Config};
use crate::portal;

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn log(msg: &str) {
    println!("[{}] {}", now(), msg);
}

fn pid_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("ruc-auth.pid")
}

fn log_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("serve.log")
}

/// 若 pid 文件指向的进程仍存活则返回其 pid
fn running_pid(pid_file: &Path) -> Option<u32> {
    let s = fs::read_to_string(pid_file).ok()?;
    let pid: u32 = s.trim().parse().ok()?;
    if Path::new(&format!("/proc/{}", pid)).exists() {
        Some(pid)
    } else {
        None
    }
}

/// 通过 check_url 判断是否联网 (2xx 视为在线; 被劫持/重定向/请求失败视为掉线)
fn check_online(cfg: &Config) -> bool {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client.get(&cfg.check_url).send() {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

fn try_relogin(cfg: &Config) {
    match portal::login(cfg) {
        Ok(()) => log("重新登录成功"),
        Err(e) => log(&format!("重新登录失败: {}", e)),
    }
}

fn serve_loop(cfg: &Config) -> Result<(), Box<dyn Error>> {
    log(&format!(
        "serve 启动 (检测间隔 {}s, 检测地址 {})",
        cfg.interval, cfg.check_url
    ));
    if check_online(cfg) {
        log("当前网络在线");
    } else {
        log("启动时检测到未联网, 尝试登录 ...");
        try_relogin(cfg);
    }
    loop {
        thread::sleep(Duration::from_secs(cfg.interval.max(5)));
        if check_online(cfg) {
            continue;
        }
        log("检测到掉线, 尝试重新登录 ...");
        try_relogin(cfg);
    }
}

/// `ruc-auth serve` 子命令入口
pub fn serve_cmd(
    config_path: &Path,
    interval: Option<u64>,
    detach: bool,
) -> Result<(), Box<dyn Error>> {
    let mut cfg = config::load(config_path)?;
    if let Some(i) = interval {
        cfg.interval = i;
    }
    if cfg.username.is_empty() || cfg.password.is_empty() {
        return Err(
            "用户名或密码未配置, 请先执行:\n  ruc-auth config set username <学号>\n  ruc-auth config set password <密码>"
                .into(),
        );
    }

    if detach {
        let pid_file = pid_path(config_path);
        if let Some(pid) = running_pid(&pid_file) {
            return Err(format!("ruc-auth serve 已在后台运行 (pid {})", pid).into());
        }
        // 重新拉起自身为独立进程组的后台进程, 日志重定向到 serve.log
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path(config_path))?;
        let child = Command::new(std::env::current_exe()?)
            .arg("--config")
            .arg(config_path)
            .arg("serve")
            .arg("--interval")
            .arg(cfg.interval.to_string())
            .stdout(log_file.try_clone()?)
            .stderr(log_file)
            .process_group(0)
            .spawn()?;
        fs::write(&pid_file, child.id().to_string())?;
        println!(
            "[+] serve 已转入后台 (pid {}), 日志: {}",
            child.id(),
            log_path(config_path).display()
        );
        return Ok(());
    }

    serve_loop(&cfg)
}
