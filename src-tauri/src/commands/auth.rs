use base64::Engine;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 获取本机 IP 地址（跨平台：通过 UDP socket 获取）
fn get_local_ip() -> Result<String, String> {
    // 桌面端使用 local-ip-address crate
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let ip =
            local_ip_address::local_ip().map_err(|e| format!("无法获取网络信息: {}", e))?;
        return Ok(ip.to_string());
    }

    // 移动端通过 UDP socket 获取本地 IP
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("无法绑定socket: {}", e))?;
        socket
            .connect("10.2.5.251:80")
            .map_err(|e| format!("无法获取网络信息: {}", e))?;
        let addr = socket
            .local_addr()
            .map_err(|e| format!("无法获取地址: {}", e))?;
        Ok(addr.ip().to_string())
    }
}

/// 获取本机 MAC 地址（去除分隔符，小写）
fn get_mac_address() -> Result<String, String> {
    // 桌面端使用 mac_address crate
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let mac = mac_address::get_mac_address()
            .map_err(|e| format!("无法获取MAC地址: {}", e))?
            .ok_or_else(|| "未找到网络接口".to_string())?;
        return Ok(mac
            .to_string()
            .replace(':', "")
            .replace('-', "")
            .to_lowercase());
    }

    // 移动端尝试从 sysfs 读取 MAC 地址
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        // 尝试读取 wlan0 接口的 MAC 地址
        let paths = [
            "/sys/class/net/wlan0/address",
            "/sys/class/net/eth0/address",
            "/sys/class/net/rmnet_data0/address",
        ];
        for path in &paths {
            if let Ok(mac) = std::fs::read_to_string(path) {
                let mac = mac.trim();
                if !mac.is_empty() && mac != "00:00:00:00:00:00" {
                    return Ok(mac.replace(':', "").replace('-', "").to_lowercase());
                }
            }
        }
        Err("无法获取MAC地址：Android系统限制了MAC地址访问。请确保已连接校园网WiFi。".to_string())
    }
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
