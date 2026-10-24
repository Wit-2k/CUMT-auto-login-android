use crate::commands::http;
use base64::Engine;
use serde_json::Value;
use std::net::IpAddr;
#[cfg(any(test, target_os = "android"))]
use std::net::Ipv4Addr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(target_os = "android")]
use std::{ffi::CStr, ptr};

fn normalize_mac(mac: &str) -> Option<String> {
    let normalized = mac.trim().replace([':', '-'], "").to_lowercase();
    let invalid = normalized.is_empty()
        || normalized == "000000000000"
        || normalized == "020000000000"
        || normalized.len() != 12
        || !normalized.chars().all(|c| c.is_ascii_hexdigit());

    if invalid {
        None
    } else {
        Some(normalized)
    }
}

/// 获取本机 IP 地址（跨平台：通过 UDP socket 获取）
pub(crate) fn get_local_ip() -> Result<String, String> {
    // 桌面端使用 local-ip-address crate
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let ip = local_ip_address::local_ip().map_err(|e| format!("无法获取网络信息: {}", e))?;
        Ok(ip.to_string())
    }

    // 移动端只接受 WLAN 地址，避免移动数据导致“已联网”的假成功。
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        get_wlan_ipv4()
    }
}

#[cfg(any(test, target_os = "android"))]
fn select_wlan_ipv4(interfaces: Vec<(String, Ipv4Addr)>) -> Option<String> {
    interfaces
        .into_iter()
        .find(|(name, ip)| {
            name == "wlan0" && !ip.is_unspecified() && !ip.is_loopback() && !ip.is_link_local()
        })
        .map(|(_, ip)| ip.to_string())
}

#[cfg(target_os = "android")]
fn android_ipv4_interfaces() -> Result<Vec<(String, Ipv4Addr)>, String> {
    let mut ifap: *mut libc::ifaddrs = ptr::null_mut();
    let rc = unsafe { libc::getifaddrs(&mut ifap) };
    if rc != 0 {
        return Err(format!(
            "无法检查WLAN网络: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut interfaces = Vec::new();
    let mut current = ifap;
    while !current.is_null() {
        let ifa = unsafe { &*current };
        if !ifa.ifa_addr.is_null() {
            let family = unsafe { (*ifa.ifa_addr).sa_family as i32 };
            if family == libc::AF_INET {
                let name = unsafe { CStr::from_ptr(ifa.ifa_name) }
                    .to_string_lossy()
                    .into_owned();
                let sockaddr = ifa.ifa_addr as *const libc::sockaddr_in;
                let raw_addr = unsafe { (*sockaddr).sin_addr.s_addr };
                let ip = Ipv4Addr::from(u32::from_be(raw_addr));
                interfaces.push((name, ip));
            }
        }
        current = ifa.ifa_next;
    }

    unsafe { libc::freeifaddrs(ifap) };
    Ok(interfaces)
}

#[cfg(target_os = "android")]
fn get_wlan_ipv4() -> Result<String, String> {
    select_wlan_ipv4(android_ipv4_interfaces()?)
        .ok_or_else(|| "未检测到WLAN IPv4地址，请连接校园网WiFi后重试。".to_string())
}

#[cfg(target_os = "ios")]
fn get_wlan_ipv4() -> Result<String, String> {
    Err("iOS 暂不支持自动选择 WLAN IPv4 地址。".to_string())
}

/// 获取本机 MAC 地址（去除分隔符，小写）
fn get_mac_address() -> Result<String, String> {
    // 桌面端使用 mac_address crate
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let mac = mac_address::get_mac_address()
            .map_err(|e| format!("无法获取MAC地址: {}", e))?
            .ok_or_else(|| "未找到网络接口".to_string())?;
        normalize_mac(&mac.to_string()).ok_or_else(|| "获取到的MAC地址无效".to_string())
    }

    // 移动端尽力从 sysfs 读取 MAC 地址；读取失败时不阻塞登录，让 Portal 返回明确结果。
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let paths = [
            "/sys/class/net/wlan0/address",
            "/sys/class/net/eth0/address",
            "/sys/class/net/rmnet_data0/address",
        ];
        for path in &paths {
            if let Ok(mac) = std::fs::read_to_string(path) {
                if let Some(normalized) = normalize_mac(&mac) {
                    return Ok(normalized);
                }
            }
        }
        Ok(String::new())
    }
}

fn extract_jsonp_payload(text: &str) -> Result<&str, String> {
    let text = text.trim();
    let start = text
        .find('(')
        .ok_or_else(|| format!("解析服务器响应失败: {}", text))?;
    let end = text
        .rfind(')')
        .filter(|end| *end > start)
        .ok_or_else(|| format!("解析服务器响应失败: {}", text))?;

    Ok(&text[start + 1..end])
}

fn decode_message(message: &str) -> String {
    match base64::engine::general_purpose::STANDARD.decode(message) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => message.to_string(),
    }
}

fn truthy_success_value(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_i64() == Some(1) || value.as_u64() == Some(1),
        Value::String(value) => {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "success" | "ok")
        }
        _ => false,
    }
}

fn value_equals_code(value: &Value, expected: &str) -> bool {
    match value {
        Value::Number(value) => value.to_string() == expected,
        Value::String(value) => value.trim() == expected,
        _ => false,
    }
}

fn is_already_online_response(result: &Value) -> bool {
    result
        .get("ret_code")
        .map(|value| value_equals_code(value, "2"))
        .unwrap_or(false)
        || result
            .get("code")
            .map(|value| value_equals_code(value, "2"))
            .unwrap_or(false)
}

fn message_contains_success(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    message.contains("认证成功")
        || message.contains("登录成功")
        || message.contains("已在线")
        || message.contains("已经在线")
        || message.contains("已登录")
        || message.contains("已经登录")
        || lower.contains("success")
        || lower.contains("already online")
        || lower.contains("already logged")
}

fn is_login_success(result: &Value, raw_message: &str, decoded_message: &str) -> bool {
    message_contains_success(raw_message)
        || message_contains_success(decoded_message)
        || result
            .get("result")
            .map(truthy_success_value)
            .unwrap_or(false)
        || result
            .get("success")
            .map(truthy_success_value)
            .unwrap_or(false)
        || result.get("ok").map(truthy_success_value).unwrap_or(false)
        || is_already_online_response(result)
}

fn extract_response_message(result: &Value) -> String {
    const MESSAGE_KEYS: &[&str] = &[
        "msg",
        "message",
        "ret_msg",
        "error_msg",
        "errorMessage",
        "desc",
        "info",
        "data",
    ];

    for key in MESSAGE_KEYS {
        if let Some(message) = result.get(*key).and_then(|v| v.as_str()) {
            if !message.trim().is_empty() {
                return message.to_string();
            }
        }
    }

    String::new()
}

fn response_summary(result: &Value) -> String {
    let Some(object) = result.as_object() else {
        return "服务器未返回消息。".to_string();
    };

    let values = ["result", "success", "ok", "ret_code", "code"]
        .into_iter()
        .filter_map(|key| result.get(key).map(|value| format!("{}={}", key, value)))
        .collect::<Vec<_>>();

    if values.is_empty() {
        let keys = object.keys().cloned().collect::<Vec<_>>().join(", ");
        format!("服务器未返回消息（响应字段：{}）。", keys)
    } else {
        format!("服务器未返回消息（{}）。", values.join(", "))
    }
}

fn parse_login_response(text: &str) -> Result<(bool, String), String> {
    let json_str = extract_jsonp_payload(text)?;
    let result: Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON解析失败: {}", e))?;

    let message = extract_response_message(&result);
    let decoded_message = decode_message(&message);
    let success = is_login_success(&result, &message, &decoded_message);
    let display_message = if decoded_message.trim().is_empty() {
        if is_already_online_response(&result) {
            "已在线，无需重复登录".to_string()
        } else if success {
            "认证成功".to_string()
        } else {
            response_summary(&result)
        }
    } else {
        decoded_message
    };

    Ok((success, display_message))
}

/// 执行 Portal 认证登录
///
/// 返回: (是否成功, 消息)
#[tauri::command]
pub async fn perform_login(
    username: String,
    password: String,
    operator: String,
) -> Result<(bool, String), String> {
    tauri::async_runtime::spawn_blocking(move || {
        perform_login_blocking(username, password, operator)
    })
    .await
    .map_err(|e| format!("登录任务失败: {}", e))?
}

fn perform_login_blocking(
    username: String,
    password: String,
    operator: String,
) -> Result<(bool, String), String> {
    let full_account = format!("{}{}", username, operator);

    // 获取网络信息
    let user_ip = get_local_ip()?;
    let user_mac = get_mac_address()?;
    let local_addr = user_ip
        .parse::<IpAddr>()
        .map_err(|e| format!("本机IP地址无效: {}", e))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let callback = format!("dr{}", timestamp);
    let timestamp_param = timestamp.to_string();

    // 构建请求参数
    let params = [
        ("c", "Portal"),
        ("a", "login"),
        ("callback", &callback),
        ("login_method", "1"),
        ("user_account", &full_account),
        ("user_password", &password),
        ("wlan_user_ip", &user_ip),
        ("wlan_user_mac", &user_mac),
        ("wlan_ac_ip", "10.2.4.1"),
        ("wlan_ac_name", "NAS"),
        ("jsVersion", "3.0"),
        ("_", timestamp_param.as_str()),
    ];

    let response = http::get_text(
        "http://10.2.5.251:801/eportal/",
        &params,
        local_addr,
        Duration::from_secs(5),
        "Mozilla/5.0",
    )
    .map_err(|_| "网络请求失败，认证服务器无响应。".to_string())?;

    parse_login_response(&response.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_jsonp_payload() {
        assert_eq!(
            extract_jsonp_payload(r#"dr123({"result":"1","msg":"ok"})"#).unwrap(),
            r#"{"result":"1","msg":"ok"}"#
        );
    }

    #[test]
    fn decodes_base64_message() {
        assert_eq!(decode_message("6K6k6K+B5oiQ5Yqf"), "认证成功");
    }

    #[test]
    fn success_uses_result_field() {
        let (success, message) =
            parse_login_response(r#"dr123({"result":"1","msg":"ok"})"#).unwrap();
        assert!(success);
        assert_eq!(message, "ok");
    }

    #[test]
    fn success_accepts_numeric_result_field() {
        let (success, message) = parse_login_response(r#"dr123({"result":1,"msg":""})"#).unwrap();
        assert!(success);
        assert_eq!(message, "认证成功");
    }

    #[test]
    fn success_accepts_boolean_success_field() {
        let (success, message) =
            parse_login_response(r#"dr123({"success":true,"message":"ok"})"#).unwrap();
        assert!(success);
        assert_eq!(message, "ok");
    }

    #[test]
    fn success_uses_decoded_message() {
        let (success, message) =
            parse_login_response(r#"dr123({"result":"0","msg":"6K6k6K+B5oiQ5Yqf"})"#).unwrap();
        assert!(success);
        assert_eq!(message, "认证成功");
    }

    #[test]
    fn success_accepts_already_online_message() {
        let (success, message) =
            parse_login_response(r#"dr123({"result":"0","msg":"用户已在线"})"#).unwrap();
        assert!(success);
        assert_eq!(message, "用户已在线");
    }

    #[test]
    fn empty_failure_message_returns_response_summary() {
        let (success, message) =
            parse_login_response(r#"dr123({"result":0,"msg":"","ret_code":3})"#).unwrap();
        assert!(!success);
        assert!(message.contains("result=0"));
        assert!(message.contains("ret_code=3"));
    }

    #[test]
    fn already_online_code_is_success() {
        let (success, message) =
            parse_login_response(r#"dr123({"result":"0","msg":"","ret_code":"2"})"#).unwrap();
        assert!(success);
        assert_eq!(message, "已在线，无需重复登录");
    }

    #[test]
    fn normalizes_valid_mac() {
        assert_eq!(normalize_mac("AA:BB:CC:00:11:22").unwrap(), "aabbcc001122");
    }

    #[test]
    fn rejects_placeholder_mac() {
        assert!(normalize_mac("02:00:00:00:00:00").is_none());
        assert!(normalize_mac("00:00:00:00:00:00").is_none());
    }

    #[test]
    fn selects_wlan_ipv4_from_interfaces() {
        let interfaces = vec![
            ("ccmni1".to_string(), Ipv4Addr::new(10, 0, 0, 5)),
            ("wlan0".to_string(), Ipv4Addr::new(10, 23, 45, 67)),
        ];

        assert_eq!(select_wlan_ipv4(interfaces).unwrap(), "10.23.45.67");
    }

    #[test]
    fn missing_or_invalid_wlan_ipv4_returns_none() {
        assert!(
            select_wlan_ipv4(vec![("ccmni1".to_string(), Ipv4Addr::new(10, 0, 0, 5))]).is_none()
        );

        assert!(select_wlan_ipv4(vec![("wlan0".to_string(), Ipv4Addr::UNSPECIFIED)]).is_none());

        assert!(
            select_wlan_ipv4(vec![("wlan0".to_string(), Ipv4Addr::new(169, 254, 1, 10))]).is_none()
        );
    }
}
