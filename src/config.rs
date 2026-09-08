//! 配置管理: ~/.config/ruc-auth/config.yaml (git config 风格的 get / set / init)
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ConfigAction;

pub const KEYS: &[&str] = &[
    "portal",
    "username",
    "password",
    "ac_id",
    "timeout",
    "interval",
    "check_url",
];

fn d_portal() -> String {
    "https://go.ruc.edu.cn".into()
}
fn d_ac_id() -> String {
    "6".into()
}
fn d_timeout() -> u64 {
    15
}
fn d_interval() -> u64 {
    30
}
fn d_check_url() -> String {
    "http://connect.rom.miui.com/generate_204".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 认证服务器地址
    #[serde(default = "d_portal")]
    pub portal: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "d_ac_id")]
    pub ac_id: String,
    /// HTTP 超时 (秒)
    #[serde(default = "d_timeout")]
    pub timeout: u64,
    /// serve 模式的网络检测间隔 (秒)
    #[serde(default = "d_interval")]
    pub interval: u64,
    /// serve 模式的联网检测地址 (期望返回 2xx)
    #[serde(default = "d_check_url")]
    pub check_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            portal: d_portal(),
            username: String::new(),
            password: String::new(),
            ac_id: d_ac_id(),
            timeout: d_timeout(),
            interval: d_interval(),
            check_url: d_check_url(),
        }
    }
}

pub fn default_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                .join(".config")
        })
        .join("ruc-auth")
        .join("config.yaml")
}

/// 确保配置文件存在, 不存在则写入默认配置; 返回是否新建
pub fn ensure(path: &Path) -> Result<bool, Box<dyn Error>> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_yaml::to_string(&Config::default())?)?;
    Ok(true)
}

pub fn load(path: &Path) -> Result<Config, Box<dyn Error>> {
    ensure(path)?;
    let raw = fs::read_to_string(path)?;
    let cfg: Config = serde_yaml::from_str(&raw)
        .map_err(|e| format!("解析配置文件 {} 失败: {}", path.display(), e))?;
    Ok(cfg)
}

pub fn save(path: &Path, cfg: &Config) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_yaml::to_string(cfg)?)?;
    Ok(())
}

pub fn get_value(cfg: &Config, key: &str) -> Option<String> {
    Some(match key {
        "portal" => cfg.portal.clone(),
        "username" => cfg.username.clone(),
        "password" => cfg.password.clone(),
        "ac_id" => cfg.ac_id.clone(),
        "timeout" => cfg.timeout.to_string(),
        "interval" => cfg.interval.to_string(),
        "check_url" => cfg.check_url.clone(),
        _ => return None,
    })
}

pub fn set_value(cfg: &mut Config, key: &str, value: &str) -> Result<(), String> {
    match key {
        "portal" => cfg.portal = value.to_string(),
        "username" => cfg.username = value.to_string(),
        "password" => cfg.password = value.to_string(),
        "ac_id" => cfg.ac_id = value.to_string(),
        "check_url" => cfg.check_url = value.to_string(),
        "timeout" => {
            cfg.timeout = value
                .parse()
                .map_err(|_| format!("timeout 需为正整数: {:?}", value))?;
        }
        "interval" => {
            cfg.interval = value
                .parse()
                .map_err(|_| format!("interval 需为正整数: {:?}", value))?;
        }
        _ => return Err(format!("未知配置项: {} (可用: {})", key, KEYS.join(", "))),
    }
    Ok(())
}

pub fn config_cmd(path: &Path, action: &ConfigAction) -> Result<(), Box<dyn Error>> {
    match action {
        ConfigAction::Init => {
            if ensure(path)? {
                println!("[config] 已创建配置文件: {}", path.display());
            } else {
                println!("[config] 配置文件已存在: {}", path.display());
            }
            println!("可用字段: {}", KEYS.join(", "));
            println!("示例: ruc-auth config set username 2020202020");
        }
        ConfigAction::Get { key } => {
            let cfg = load(path)?;
            match key {
                Some(k) => match get_value(&cfg, k) {
                    Some(v) => println!("{}", v),
                    None => {
                        return Err(format!(
                            "未知配置项: {} (可用: {})",
                            k,
                            KEYS.join(", ")
                        )
                        .into())
                    }
                },
                None => print!("{}", serde_yaml::to_string(&cfg)?),
            }
        }
        ConfigAction::Set { key, value } => {
            let mut cfg = load(path)?;
            set_value(&mut cfg, key, value).map_err(|e| -> Box<dyn Error> { e.into() })?;
            save(path, &cfg)?;
            println!("[config] {} = {}", key, get_value(&cfg, key).unwrap_or_default());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_roundtrip() {
        let mut cfg = Config::default();
        set_value(&mut cfg, "username", "u1").unwrap();
        set_value(&mut cfg, "timeout", "20").unwrap();
        assert_eq!(get_value(&cfg, "username").unwrap(), "u1");
        assert_eq!(get_value(&cfg, "timeout").unwrap(), "20");
        assert!(set_value(&mut cfg, "timeout", "abc").is_err());
        assert!(set_value(&mut cfg, "nope", "x").is_err());
    }
}
