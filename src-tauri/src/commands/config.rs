use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

/// 登录配置（与 Python 版本 config.json 格式完全兼容）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginConfig {
    pub username: String,
    pub password: String,
    pub operator: String,
}

/// 获取配置文件路径。
///
/// 桌面端和 Android 都使用 Tauri 的应用配置目录，避免在 Android 上写 APK/native lib 目录。
fn get_config_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法获取配置目录: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {}", e))?;
    Ok(dir.join("config.json"))
}

fn save_config_to_path(path: PathBuf, config: &LoginConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("写入配置文件失败: {}", e))
}

fn load_config_from_path(path: PathBuf) -> Option<LoginConfig> {
    if !path.exists() {
        return None;
    }
    let json = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

/// 保存登录配置到 JSON 文件
#[tauri::command]
pub fn save_config<R: Runtime>(app: AppHandle<R>, config: LoginConfig) -> Result<(), String> {
    let path = get_config_path(&app)?;
    save_config_to_path(path, &config)
}

/// 从 JSON 文件加载登录配置
#[tauri::command]
pub fn load_config<R: Runtime>(app: AppHandle<R>) -> Option<LoginConfig> {
    let path = get_config_path(&app).ok()?;
    load_config_from_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_config_path() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cumt-auto-login-config-{timestamp}.json"))
    }

    #[test]
    fn save_and_load_config_round_trip() {
        let path = unique_config_path();
        let config = LoginConfig {
            username: "01234567".to_string(),
            password: "secret".to_string(),
            operator: "@telecom".to_string(),
        };

        save_config_to_path(path.clone(), &config).expect("config should save");
        let loaded = load_config_from_path(path.clone()).expect("config should load");

        assert_eq!(loaded.username, config.username);
        assert_eq!(loaded.password, config.password);
        assert_eq!(loaded.operator, config.operator);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_missing_config_returns_none() {
        assert!(load_config_from_path(unique_config_path()).is_none());
    }
}
