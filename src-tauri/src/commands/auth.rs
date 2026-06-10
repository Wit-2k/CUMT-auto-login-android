use base64::Engine;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 获取本机 MAC 地址（去除分隔符，小写）
fn get_mac_address() -> Result<String, String> {
    let mac = mac_address::get_mac_address()
        .map_err(|e| format!("无法获取MAC地址: {}", e))?
        .ok_or_else(|| "未找到网络接口".to_string())?;
    Ok(mac.to_string().replace(':', "").replace('-', "").to_lowercase())
}

/// 获取本机 IP 地址
fn get_local_ip() -> Result<String, String> {
    let ip = local_ip_address::local_ip().map_err(|e| format!("无法获取网络信息: {}", e))?;
    Ok(ip.to_string())
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
    let full_account = format!("{}{}", username, operator);

    // 获取网络信息
    let user_ip = get_local_ip()?;
    let user_mac = get_mac_address()?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let callback = format!("dr{}", timestamp);

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
        ("_", &timestamp.to_string()),
    ];

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

    let response = client
        .get("http://10.2.5.251:801/eportal/")
        .query(&params)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|_| "网络请求失败，认证服务器无响应。".to_string())?;

    let text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    // 解析 JSONP 响应: dr1234567890({...})
    let re = Regex::new(r"\((.*)\)").map_err(|e| format!("正则表达式错误: {}", e))?;
    let json_str = re
        .captures(&text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| format!("解析服务器响应失败: {}", text))?;

    let result: Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON解析失败: {}", e))?;

    let message = result
        .get("msg")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // 尝试 Base64 解码消息
    let decoded_message = match base64::engine::general_purpose::STANDARD.decode(&message) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => message.clone(),
    };

    let success = message.contains("认证成功")
        || result
            .get("result")
            .and_then(|v| v.as_str())
            .map(|s| s == "1")
            .unwrap_or(false);

    Ok((success, decoded_message))
}
