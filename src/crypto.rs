//! 深澜 srun_bx1 协议加密部分: XXTEA / 自定义 Base64 / HMAC-MD5 / SHA1
use hmac::{Hmac, Mac};
use sha1::{Digest, Sha1};

const BASE64_ALPHABET: &[u8; 64] = b"LVoJPiCN2R8G90yg+hmFHuacZ1OWMnrsSTXkYpUq/3dlbfKwv6xztjI7DeBE45QA";
const STD_B64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const SRUN_PREFIX: &str = "{SRBX1}";
const DELTA: u32 = 0x9E37_79B9;

/// 按 Unicode 码点(而非 UTF-8 字节)以小端序打包为 32 位字, 与 JS/Python 参考实现一致
fn str_to_words(s: &str, append_len: bool) -> Vec<u32> {
    let units: Vec<u32> = s.chars().map(|c| c as u32).collect();
    let mut words = Vec::with_capacity(units.len() / 4 + 2);
    let mut i = 0;
    while i < units.len() {
        let mut w: u32 = 0;
        for j in 0..4 {
            if i + j < units.len() {
                w |= units[i + j].wrapping_shl((8 * j) as u32);
            }
        }
        words.push(w);
        i += 4;
    }
    if append_len {
        words.push(units.len() as u32);
    }
    words
}

#[cfg_attr(not(test), allow(dead_code))]
fn bytes_to_words(b: &[u8]) -> Vec<u32> {
    b.chunks(4)
        .map(|c| {
            let mut w = [0u8; 4];
            w[..c.len()].copy_from_slice(c);
            u32::from_le_bytes(w)
        })
        .collect()
}

fn words_to_bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

pub fn xxttea_encrypt(data: &str, key: &str) -> Vec<u8> {
    let mut v = str_to_words(data, true);
    let mut k = str_to_words(key, false);
    while k.len() < 4 {
        k.push(0);
    }
    let n = v.len() - 1;
    let q = 6 + 52 / (n + 1);
    let mut z = v[n];
    let mut d: u32 = 0;
    for _ in 0..q {
        d = d.wrapping_add(DELTA);
        let e = ((d >> 2) & 3) as usize;
        for p in 0..n {
            let y = v[p + 1];
            let mut m = (z >> 5) ^ y.wrapping_shl(2);
            m = m.wrapping_add((y >> 3) ^ z.wrapping_shl(4) ^ (d ^ y));
            m = m.wrapping_add(k[(p & 3) ^ e] ^ z);
            v[p] = v[p].wrapping_add(m);
            z = v[p];
        }
        let y = v[0];
        let mut m = (z >> 5) ^ y.wrapping_shl(2);
        m = m.wrapping_add((y >> 3) ^ z.wrapping_shl(4) ^ (d ^ y));
        m = m.wrapping_add(k[(n & 3) ^ e] ^ z);
        v[n] = v[n].wrapping_add(m);
        z = v[n];
    }
    words_to_bytes(&v)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn xxttea_decrypt(data: &[u8], key: &str) -> String {
    let mut v = bytes_to_words(data);
    let mut k = str_to_words(key, false);
    while k.len() < 4 {
        k.push(0);
    }
    let n = v.len() - 1;
    let q = 6 + 52 / (n + 1);
    let mut y = v[0];
    let mut d = (q as u32).wrapping_mul(DELTA);
    while d != 0 {
        let e = ((d >> 2) & 3) as usize;
        for p in (1..=n).rev() {
            let z = v[p - 1];
            let mut m = (z >> 5) ^ y.wrapping_shl(2);
            m = m.wrapping_add((y >> 3) ^ z.wrapping_shl(4) ^ (d ^ y));
            m = m.wrapping_add(k[(p & 3) ^ e] ^ z);
            v[p] = v[p].wrapping_sub(m);
            y = v[p];
        }
        let z = v[n];
        let mut m = (z >> 5) ^ y.wrapping_shl(2);
        m = m.wrapping_add((y >> 3) ^ z.wrapping_shl(4) ^ (d ^ y));
        m = m.wrapping_add(k[e] ^ z);
        v[0] = v[0].wrapping_sub(m);
        y = v[0];
        d = d.wrapping_sub(DELTA);
    }
    let n = *v.last().unwrap() as usize;
    let raw = words_to_bytes(&v);
    // latin-1 解码 (与参考实现行为一致)
    raw[..n.min(raw.len())].iter().map(|&b| b as char).collect()
}

// ---------- Base64 (标准编码后按自定义字母表替换) ----------

fn base64_std(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(STD_B64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(STD_B64_ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            STD_B64_ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            STD_B64_ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn custom_b64(data: &[u8]) -> String {
    base64_std(data)
        .bytes()
        .map(|b| match STD_B64_ALPHABET.iter().position(|&c| c == b) {
            Some(pos) => BASE64_ALPHABET[pos] as char,
            // '=' 等 padding 字符不参与替换, 与参考实现 str.maketrans 行为一致
            None => b as char,
        })
        .collect()
}

// ---------- 协议字段 ----------

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 与 Python json.dumps 默认分隔符(", " / ": ")保持一致
pub fn encode_user_info(username: &str, password: &str, ip: &str, acid: &str, token: &str) -> String {
    let info = format!(
        "{{\"username\": {}, \"password\": {}, \"ip\": {}, \"acid\": {}, \"enc_ver\": \"srun_bx1\"}}",
        serde_json::to_string(username).unwrap(),
        serde_json::to_string(password).unwrap(),
        serde_json::to_string(ip).unwrap(),
        serde_json::to_string(acid).unwrap()
    );
    let enc = xxttea_encrypt(&info, token);
    format!("{}{}", SRUN_PREFIX, custom_b64(&enc))
}

pub fn hmd5(token: &str, password: &str) -> String {
    let mut mac = <Hmac<md5::Md5>>::new_from_slice(token.as_bytes()).unwrap();
    mac.update(password.as_bytes());
    hex(&mac.finalize().into_bytes())
}

pub fn chksum(token: &str, username: &str, hmd5_hex: &str, ac_id: &str, ip: &str, n: u32, typ: u32, info: &str) -> String {
    let s = format!(
        "{t}{u}{t}{h}{t}{a}{t}{i}{t}{n}{t}{ty}{t}{inf}",
        t = token, u = username, h = hmd5_hex, a = ac_id, i = ip, n = n, ty = typ, inf = info
    );
    let mut hasher = Sha1::new();
    hasher.update(s.as_bytes());
    hex(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 与 Python 参考实现 (portal_login/portal_login.py) 逐字节一致的验证向量
    #[test]
    fn matches_python_reference_vectors() {
        let info = "{\"username\": \"test\", \"password\": \"123456\", \"ip\": \"10.10.17.47\", \"acid\": \"6\", \"enc_ver\": \"srun_bx1\"}";
        for (token, expected) in [
            ("abc", "v866JZYz1ie07ise+z1hPkG2KmfbCQyeCQkahnd39gdxkHrPCbUPdsPS+8VGCus7kqHBKqI141cfehLV1ZwTZeng978KbXoFz1VEnvPC/d3k3FFuFWh946gFqxmvhXpa7/k2y8UeeJP="),
            ("challenge1234567890", "9o+5XfoSbJk0bP2kFF8MaxTSP8xffLDUMB8tfdVDuo/NMOhPfcJRZFj+cwFP+70kwrdQJGzWhsyKYvMvduixkkuPhu6UXPsv39PW+sKLbCwDGFOueL8HXaqyjzRssKIjs0JNIKeHkp4="),
        ] {
            let got = custom_b64(&xxttea_encrypt(info, token));
            assert_eq!(got, expected, "token={}", token);
        }
        assert_eq!(hmd5("k", "p"), "8251ba1ec2cd47f553ffa23e1b161572");
        assert_eq!(
            chksum("tok", "user", "deadbeef", "6", "1.2.3.4", 200, 1, "INFO"),
            "78facec354867478e18c57ae10189eae20e99896"
        );
    }

    #[test]
    fn xxtea_roundtrip() {
        for (user, pwd) in [
            ("test", "123456"),
            ("20232000123", "Passw0rd!@#"),
            ("dl@ruc", "abc123!@#"),
            ("", "x"),
        ] {
            let info = format!(
                "{{\"username\": {}, \"password\": {}, \"ip\": \"10.10.17.47\", \"acid\": \"6\", \"enc_ver\": \"srun_bx1\"}}",
                serde_json::to_string(user).unwrap(),
                serde_json::to_string(pwd).unwrap()
            );
            for token in ["abc", "challenge1234567890"] {
                let enc = xxttea_encrypt(&info, token);
                assert_eq!(xxttea_decrypt(&enc, token), info);
                assert_eq!(xxttea_encrypt(&info, token), enc, "not deterministic");
            }
        }
    }
}
