use reqwest::Client;
use std::time::Duration;

/// 检查是否已连接到互联网
#[tauri::command]
pub async fn check_online() -> bool {
    let client = match Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client
        .get("http://detectportal.firefox.com/success.txt")
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
