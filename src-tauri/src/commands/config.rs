use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 登录配置（与 Python 版本 config.json 格式完全兼容）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginConfig {
    pub username: String,
    pub password: String,
    pub operator: String,
}

/// 获取配置文件路径（与可执行文件同目录下的 config.json）
fn get_config_path() -> PathBuf {
    let exe_path = std::env::current_exe().unwrap_or_default();
    let dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    dir.join("config.json")
}

/// 保存登录配置到 JSON 文件
#[tauri::command]
pub fn save_config(config: LoginConfig) -> Result<(), String> {
    let path = get_config_path();
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(())
}

/// 从 JSON 文件加载登录配置
#[tauri::command]
pub fn load_config() -> Option<LoginConfig> {
    let path = get_config_path();
    if !path.exists() {
        return None;
    }
    let json = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}
