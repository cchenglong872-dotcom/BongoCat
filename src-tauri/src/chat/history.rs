use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

pub const HISTORY_FILE: &str = "chat-history.json";
pub const MAX_MESSAGES: usize = 50;
pub const MAX_CONTEXT_MESSAGES: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub timestamp: i64,
}

fn history_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(HISTORY_FILE))
        .map_err(|err| format!("无法获取应用配置目录: {err}"))
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> Vec<ChatMessage> {
    match history_path(app) {
        Ok(path) => load_from(&path),
        Err(_) => Vec::new(),
    }
}

fn load_from(path: &Path) -> Vec<ChatMessage> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let Ok(mut messages) = serde_json::from_str::<Vec<ChatMessage>>(&content) else {
        return Vec::new();
    };

    if messages.len() > MAX_MESSAGES {
        messages = messages[messages.len() - MAX_MESSAGES..].to_vec();
    }

    messages
}

pub fn append<R: Runtime>(app: &AppHandle<R>, message: ChatMessage) -> Result<(), String> {
    let mut messages = load(app);

    messages.push(message);

    if messages.len() > MAX_MESSAGES {
        messages = messages[messages.len() - MAX_MESSAGES..].to_vec();
    }

    save(app, &messages)
}

pub fn clear<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    save(app, &[])
}

pub fn save<R: Runtime>(app: &AppHandle<R>, messages: &[ChatMessage]) -> Result<(), String> {
    let path = history_path(app)?;

    save_to(&path, messages)
}

fn save_to(path: &Path, messages: &[ChatMessage]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("无法创建数据目录: {err}"))?;
    }

    let content = serde_json::to_string_pretty(messages)
        .map_err(|err| format!("序列化聊天记录失败: {err}"))?;

    fs::write(path, content).map_err(|err| format!("无法写入聊天记录: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_file() -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);

        temp_dir().join(format!("bongo-chat-test-{id}.json"))
    }

    fn message(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let path = temp_file();

        assert!(load_from(&path).is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_file();
        let messages = vec![message("user", "你好"), message("assistant", "你好！")];

        let _ = std::fs::remove_file(&path);
        save_to(&path, &messages).unwrap();

        let loaded = load_from(&path);

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].role, "user");
        assert_eq!(loaded[1].content, "你好！");
    }

    #[test]
    fn load_trims_to_max() {
        let path = temp_file();
        let messages = (0..(MAX_MESSAGES + 10) as i64)
            .map(|i| message("user", &format!("msg-{i}")))
            .collect::<Vec<_>>();

        let _ = std::fs::remove_file(&path);
        save_to(&path, &messages).unwrap();

        let loaded = load_from(&path);

        assert_eq!(loaded.len(), MAX_MESSAGES);
        assert_eq!(loaded[0].content, "msg-10");
    }

    #[test]
    fn load_corrupted_file_returns_empty() {
        let path = temp_file();
        std::fs::write(&path, "{ not valid json").unwrap();

        assert!(load_from(&path).is_empty());
    }
}
