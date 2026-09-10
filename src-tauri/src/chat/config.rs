use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

pub const CONFIG_FILE: &str = ".env";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub reasoning: bool,
    pub user_avatar_path: Option<String>,
    pub weather_province: String,
    pub weather_city: String,
    pub weather_api_key: String,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-v4-pro".to_string(),
            reasoning: true,
            user_avatar_path: None,
            weather_province: String::new(),
            weather_city: String::new(),
            weather_api_key: String::new(),
        }
    }
}

fn config_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(CONFIG_FILE))
        .map_err(|err| format!("无法获取应用配置目录: {err}"))
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> ChatConfig {
    let Ok(path) = config_path(app) else {
        return ChatConfig::default();
    };

    let Ok(content) = fs::read_to_string(&path) else {
        let default = ChatConfig::default();

        let _ = save(app, &default);

        return default;
    };

    parse_env(&content)
}

pub fn save<R: Runtime>(app: &AppHandle<R>, config: &ChatConfig) -> Result<(), String> {
    let path = config_path(app)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("无法创建配置目录: {err}"))?;
    }

    fs::write(&path, serialize_env(config)).map_err(|err| format!("无法写入配置文件: {err}"))
}

fn parse_env(content: &str) -> ChatConfig {
    let mut config = ChatConfig::default();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let value = value.trim().trim_matches('"').trim_matches('\'');

        match key.trim() {
            "LLM_API_KEY" => config.api_key = value.to_string(),
            "LLM_BASE_URL" => config.base_url = value.to_string(),
            "LLM_MODEL" => config.model = value.to_string(),
            "LLM_REASONING" => config.reasoning = value == "true" || value == "1",
            "CHAT_USER_AVATAR" => {
                config.user_avatar_path = (!value.is_empty()).then(|| value.to_string())
            }
            "WEATHER_PROVINCE" => config.weather_province = value.to_string(),
            "WEATHER_CITY" => config.weather_city = value.to_string(),
            "QWEATHER_API_KEY" | "X-QW-Api-Key" => config.weather_api_key = value.to_string(),
            _ => {}
        }
    }

    config
}

fn serialize_env(config: &ChatConfig) -> String {
    format!(
        "# BongoCat LLM 配置\n\
         # 可在设置界面或直接修改本文件后保存，重新打开聊天窗口生效。\n\
         LLM_API_KEY={}\n\
         LLM_BASE_URL={}\n\
         LLM_MODEL={}\n\
         LLM_REASONING={}\n\
         CHAT_USER_AVATAR={}\n\
         WEATHER_PROVINCE={}\n\
         WEATHER_CITY={}\n\
         QWEATHER_API_KEY={}\n",
        config.api_key,
        config.base_url,
        config.model,
        config.reasoning,
        config.user_avatar_path.as_deref().unwrap_or_default(),
        config.weather_province,
        config.weather_city,
        config.weather_api_key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_should_read_all_keys() {
        let config = parse_env(
            "# 注释\nLLM_API_KEY=sk-test-123\nLLM_BASE_URL=https://api.example.com\nLLM_MODEL=gpt-4o\nLLM_REASONING=true\n",
        );

        assert_eq!(config.api_key, "sk-test-123");
        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.model, "gpt-4o");
        assert!(config.reasoning);
    }

    #[test]
    fn parse_env_should_handle_quotes_and_missing_keys() {
        let config = parse_env("LLM_API_KEY=\"sk-quoted\"\n");

        assert_eq!(config.api_key, "sk-quoted");
        assert_eq!(config.base_url, ChatConfig::default().base_url);
        assert!(config.reasoning);
    }

    #[test]
    fn parse_env_should_parse_reasoning_false() {
        let config = parse_env("LLM_REASONING=false\n");

        assert!(!config.reasoning);
    }

    #[test]
    fn serialize_parse_round_trip() {
        let config = ChatConfig {
            api_key: "sk-abc".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-chat".to_string(),
            reasoning: false,
            user_avatar_path: None,
            weather_province: "广东".to_string(),
            weather_city: "深圳".to_string(),
            weather_api_key: "qw-test-456".to_string(),
        };

        let parsed = parse_env(&serialize_env(&config));

        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.base_url, config.base_url);
        assert_eq!(parsed.model, config.model);
        assert_eq!(parsed.reasoning, config.reasoning);
        assert_eq!(parsed.user_avatar_path, config.user_avatar_path);
        assert_eq!(parsed.weather_province, config.weather_province);
        assert_eq!(parsed.weather_city, config.weather_city);
        assert_eq!(parsed.weather_api_key, config.weather_api_key);
    }

    #[test]
    fn parse_env_should_read_weather_keys() {
        let config = parse_env(
            "WEATHER_PROVINCE=陕西省\nWEATHER_CITY=西安市\nQWEATHER_API_KEY=qw-123\n",
        );

        assert_eq!(config.weather_province, "陕西省");
        assert_eq!(config.weather_city, "西安市");
        assert_eq!(config.weather_api_key, "qw-123");
    }

    #[test]
    fn parse_env_should_fallback_to_legacy_qweather_key() {
        let config = parse_env("X-QW-Api-Key=legacy-key\n");

        assert_eq!(config.weather_api_key, "legacy-key");
    }

    #[test]
    fn parse_env_should_default_empty_weather_keys() {
        let config = parse_env("LLM_MODEL=deepseek-chat\n");

        assert_eq!(config.weather_province, "");
        assert_eq!(config.weather_city, "");
        assert_eq!(config.weather_api_key, "");
    }

    #[test]
    fn parse_env_should_read_avatar_path() {
        let config = parse_env("CHAT_USER_AVATAR=chat/avatar/user-avatar.png\n");

        assert_eq!(
            config.user_avatar_path.as_deref(),
            Some("chat/avatar/user-avatar.png")
        );
    }

    #[test]
    fn parse_env_should_default_empty_avatar_path() {
        let config = parse_env("CHAT_USER_AVATAR=\n");

        assert!(config.user_avatar_path.is_none());
    }
}
