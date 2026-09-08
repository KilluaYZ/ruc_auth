//! 认证门户交互: get_challenge / srun_portal 登录流程
use std::error::Error;
use std::net::UdpSocket;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::config::{self, Config};
use crate::crypto;

pub fn build_client(timeout: u64) -> Result<reqwest::blocking::Client, Box<dyn Error>> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?)
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn http_get_json(
    client: &reqwest::blocking::Client,
    url: &str,
    params: &mut Vec<(&str, String)>,
) -> Result<Value, Box<dyn Error>> {
    let cb = format!(
        "jQuery{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    );
    params.push(("callback", cb.clone()));
    let qs: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let full = format!("{}?{}", url, qs);
    let resp = client
        .get(&full)
        .header("User-Agent", "Mozilla/5.0")
        .header("Referer", "https://go.ruc.edu.cn/srun_portal_pc")
        .send()?;
    let status = resp.status();
    let body = resp.text()?;
    let mut body = body.trim().to_string();
    if !status.is_success() {
        return Err(format!("HTTP {} : {}", status, body).into());
    }
    // 剥离 JSONP 包装
    if let Some(rest) = body.strip_prefix(&cb) {
        body = rest.trim().to_string();
        if body.starts_with('(') && body.ends_with(')') && body.len() >= 2 {
            body = body[1..body.len() - 1].to_string();
        }
    }
    Ok(serde_json::from_str(&body)?)
}

pub fn get_local_ip(portal_host: &str) -> Result<String, Box<dyn Error>> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.connect((portal_host, 80))?;
    let ip = sock.local_addr()?.ip();
    Ok(ip.to_string())
}

fn check_credentials(cfg: &Config) -> Result<(), Box<dyn Error>> {
    if cfg.username.is_empty() || cfg.password.is_empty() {
        return Err(
            "用户名或密码未配置, 请先执行:\n  ruc-auth config set username <学号>\n  ruc-auth config set password <密码>"
                .into(),
        );
    }
    Ok(())
}

/// 执行一次完整登录
pub fn login(cfg: &Config) -> Result<(), Box<dyn Error>> {
    check_credentials(cfg)?;
    let portal = cfg.portal.trim_end_matches('/').to_string();
    let client = build_client(cfg.timeout)?;

    let host = reqwest::Url::parse(&portal)?
        .host_str()
        .ok_or("无法解析 portal host")?
        .to_string();
    let ip = get_local_ip(&host)?;
    println!("[*] 本机 IP: {}  账号: {}  ac_id: {}", ip, cfg.username, cfg.ac_id);

    println!("[*] 获取 challenge ...");
    let chal = http_get_json(
        &client,
        &format!("{}/cgi-bin/get_challenge", portal),
        &mut vec![
            ("username", cfg.username.clone()),
            ("ip", ip.clone()),
        ],
    )?;
    let token = chal
        .get("challenge")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    if token.is_empty() {
        return Err(format!("获取 challenge 失败: {}", chal).into());
    }

    let h = crypto::hmd5(&token, &cfg.password);
    let info = crypto::encode_user_info(&cfg.username, &cfg.password, &ip, &cfg.ac_id, &token);
    let (n, typ) = (200u32, 1u32);
    let ck = crypto::chksum(&token, &cfg.username, &h, &cfg.ac_id, &ip, n, typ, &info);

    println!("[*] 提交登录 ...");
    let resp = http_get_json(
        &client,
        &format!("{}/cgi-bin/srun_portal", portal),
        &mut vec![
            ("action", "login".to_string()),
            ("username", cfg.username.clone()),
            ("password", format!("{{MD5}}{}", h)),
            ("ac_id", cfg.ac_id.clone()),
            ("ip", ip),
            ("chksum", ck),
            ("info", info),
            ("n", n.to_string()),
            ("type", typ.to_string()),
            ("os", "Linux".to_string()),
            ("name", "Linux".to_string()),
            ("double_stack", "0".to_string()),
        ],
    )?;

    let res = resp.get("res").and_then(|v| v.as_str()).unwrap_or("");
    let error = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    if error == "ok" || res == "ok" {
        println!("[+] 登录成功!");
        Ok(())
    } else if res == "ip_already_online_error" {
        println!("[!] 该 IP 已在线 (ip_already_online_error), 无需重复登录");
        Ok(())
    } else {
        Err(format!("登录失败: {}", resp).into())
    }
}

/// `ruc-auth login` 子命令入口
pub fn login_cmd(config_path: &Path) -> Result<(), Box<dyn Error>> {
    let cfg = config::load(config_path)?;
    login(&cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_special_chars() {
        assert_eq!(urlencode("{SRBX1}abc+/="), "%7BSRBX1%7Dabc%2B%2F%3D");
        assert_eq!(urlencode("a b"), "a%20b");
    }
}
