# BongoCat LLM 对话功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 BongoCat 增加 LLM 对话能力：双击桌面猫咪弹出微信风格的聊天窗口，通过 OpenAI 兼容 API 流式对话，配置（key/base_url/model/推理开关）存于应用配置目录的 `.env` 文件。

**Architecture:** 新增独立 `chat` 窗口承载聊天 UI。Rust 端新增 `chat` 模块：`config`（.env 读写）、`history`（聊天记录 JSON 读写）、`stream`（reqwest 流式请求 + SSE 解析 + Tauri 事件推送）、`commands`（6 个 Tauri 命令）。前端通过 `plugins/chat.ts` 封装 invoke，`stores/chat.ts` 维护状态，`composables/useChat.ts` 监听 `chat-stream`/`chat-done` 事件做增量渲染。双击检测在主窗口 `src/pages/main/index.vue` 内用「300ms 内二次按下 + 位移 <5px」判定。

**Tech Stack:** Rust（reqwest 0.12 + rustls、futures-util）、Tauri 2、Vue 3、Pinia、antdv-next、UnoCSS、vue-markdown-render（已存在）。

**Spec:** 见对话中的设计方案（独立聊天窗口 / 聊天记录持久化 / 配置文件+设置界面 / Rust 后端调用）。

## Global Constraints

- 仅使用 pnpm（`preinstall` 强制）。
- **不得修改项目根目录 `D:\CL\BongoCat-master\.env`**（含用户真实 API key）。本功能在**应用配置目录**下新建独立 `.env`，键名 `LLM_API_KEY` / `LLM_BASE_URL` / `LLM_MODEL` / `LLM_REASONING`。
- 本项目**不是 git 仓库**，所有计划中的「Commit」步骤改为「Checkpoint 验证」步骤。
- 代码风格遵循 `@antfu/eslint-config`：导入自然排序、Vue 属性字母序、缩进 2 空格。
- 所有新增用户可见文案走 vue-i18n（zh-CN / zh-TW / en-US / vi-VN / pt-BR 五语言）。
- Rust 端流式请求 URL 为 `{base_url}/chat/completions`（OpenAI 兼容）。
- 消息角色限定 `user` / `assistant`；系统提示词仅拼接进请求，不落盘。

---

### Task 1: Rust 依赖（reqwest + futures-util）

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Produces: `reqwest`（json/stream/rustls-tls/http2）、`futures-util` 可用。

- [ ] **Step 1: 添加依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中（`serde_json.workspace = true` 之后）添加：

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "stream", "rustls-tls", "http2"] }
futures-util = "0.3"
```

说明：`default-features = false` + `rustls-tls` 避免 Linux 构建时对 OpenSSL 系统库的依赖；`http2` 保留 HTTP/2 支持（纯 Rust）。

- [ ] **Step 2: 验证编译**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo check`
Expected: 编译通过（首次会下载并编译 reqwest，可能需要几分钟）。

---

### Task 2: 配置模块（.env 解析/序列化 + 单测）

**Files:**
- Create: `src-tauri/src/chat/mod.rs`
- Create: `src-tauri/src/chat/config.rs`
- Modify: `src-tauri/src/lib.rs:1`

**Interfaces:**
- Produces: `ChatConfig { api_key: String, base_url: String, model: String, reasoning: bool }`，serde `camelCase`。`config::load(&AppHandle) -> ChatConfig`、`config::save(&AppHandle, &ChatConfig) -> Result<(), String>`。

- [ ] **Step 1: 写失败测试**

创建 `src-tauri/src/chat/mod.rs`（仅声明子模块，后续任务逐步追加）：

```rust
pub mod config;
```

创建 `src-tauri/src/chat/config.rs`，先只写测试（下面 Step 3 的实现后追加）：

```rust
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
        };

        let parsed = parse_env(&serialize_env(&config));

        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.base_url, config.base_url);
        assert_eq!(parsed.model, config.model);
        assert_eq!(parsed.reasoning, config.reasoning);
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo test chat::config`
Expected: 编译报错（`parse_env` 未定义、`ChatConfig` 未定义）。

- [ ] **Step 3: 最小实现**

将 `config.rs` 补全为：

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const CONFIG_FILE: &str = ".env";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub reasoning: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-v4-pro".to_string(),
            reasoning: true,
        }
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(CONFIG_FILE))
        .map_err(|err| format!("无法获取应用配置目录: {err}"))
}

pub fn load(app: &AppHandle) -> ChatConfig {
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

pub fn save(app: &AppHandle, config: &ChatConfig) -> Result<(), String> {
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
         LLM_REASONING={}\n",
        config.api_key, config.base_url, config.model, config.reasoning,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Step 1 中的测试代码原样保留
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo test chat::config`
Expected: 4 个测试全部 PASS。

- [ ] **Step 5: 注册模块**

修改 `src-tauri/src/lib.rs` 第 1 行前新增 `mod chat;`：

```rust
mod chat;
mod core;
mod utils;
```

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo check`
Expected: 编译通过。

- [ ] **Step 6: Checkpoint**

确认 `src-tauri/src/chat/config.rs` 完整、`cargo test chat::config` 全绿、`cargo check` 通过。

---

### Task 3: 聊天记录模块（JSON 读写 + 单测）

**Files:**
- Create: `src-tauri/src/chat/history.rs`
- Modify: `src-tauri/src/chat/mod.rs`（追加 `pub mod history;`）

**Interfaces:**
- Produces: `ChatMessage { role: String, content: String, timestamp: i64 }`，serde 直接序列化（字段名保持小写）。常量 `MAX_MESSAGES = 50`、`MAX_CONTEXT_MESSAGES = 30`。`history::load(&AppHandle) -> Vec<ChatMessage>`、`history::append(&AppHandle, ChatMessage) -> Result<(), String>`、`history::clear(&AppHandle) -> Result<(), String>`。测试用 `load_from(&Path)` / `save_to(&Path, &[ChatMessage])`。

- [ ] **Step 1: 写失败测试**

创建 `src-tauri/src/chat/history.rs`：

```rust
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

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
```

- [ ] **Step 2: 运行确认失败**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo test chat::history`
Expected: 编译报错（`load_from`/`save_to` 未定义）。

- [ ] **Step 3: 最小实现**

将 `history.rs` 补全（在 Step 1 内容基础上追加实现，测试保留）：

```rust
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

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

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(HISTORY_FILE))
        .map_err(|err| format!("无法获取应用配置目录: {err}"))
}

pub fn load(app: &AppHandle) -> Vec<ChatMessage> {
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

pub fn append(app: &AppHandle, message: ChatMessage) -> Result<(), String> {
    let mut messages = load(app);

    messages.push(message);

    if messages.len() > MAX_MESSAGES {
        messages = messages[messages.len() - MAX_MESSAGES..].to_vec();
    }

    save(app, &messages)
}

pub fn clear(app: &AppHandle) -> Result<(), String> {
    save(app, &[])
}

pub fn save(app: &AppHandle, messages: &[ChatMessage]) -> Result<(), String> {
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
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo test chat::history`
Expected: 4 个测试全部 PASS。

- [ ] **Step 5: 注册模块**

修改 `src-tauri/src/chat/mod.rs`：

```rust
pub mod config;
pub mod history;
```

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo check`
Expected: 编译通过。

- [ ] **Step 6: Checkpoint**

确认 `cargo test chat::history` 全绿、`cargo check` 通过。

---

### Task 4: 流式请求模块（SSE 解析 + 事件推送 + 单测）

**Files:**
- Create: `src-tauri/src/chat/stream.rs`
- Modify: `src-tauri/src/chat/mod.rs`（追加 `pub mod stream;`）

**Interfaces:**
- Produces: 常量 `EVENT_STREAM: &str = "chat-stream"`、`EVENT_DONE: &str = "chat-done"`。`StreamPayload { delta: String }`、`DonePayload { content: String, error: Option<String> }`（均 serde `camelCase`）。`stream::stream_chat(AppHandle, ChatConfig, Vec<ChatMessage>) -> Result<String, String>`。纯函数 `extract_delta(&str) -> Option<String>`。

- [ ] **Step 1: 写失败测试**

创建 `src-tauri/src/chat/stream.rs`：

```rust
use tauri::AppHandle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_delta_from_content_line() {
        let line = r#"data: {"choices":[{"delta":{"content":"你好"}}]}"#;

        assert_eq!(extract_delta(line).as_deref(), Some("你好"));
    }

    #[test]
    fn extract_delta_ignores_done() {
        assert_eq!(extract_delta("data: [DONE]"), None);
    }

    #[test]
    fn extract_delta_ignores_non_data_line() {
        assert_eq!(extract_delta(": keep-alive"), None);
        assert_eq!(extract_delta(""), None);
    }

    #[test]
    fn extract_delta_handles_crlf() {
        let line = r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#;

        assert_eq!(extract_delta(&format!("{line}\r")).as_deref(), Some("hi"));
    }

    #[test]
    fn extract_delta_returns_none_without_content() {
        let line = r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#;

        assert_eq!(extract_delta(line), None);
    }
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo test chat::stream`
Expected: 编译报错（`extract_delta` 未定义）。

- [ ] **Step 3: 最小实现**

将 `stream.rs` 补全为：

```rust
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

use super::config::ChatConfig;
use super::history::ChatMessage;

pub const EVENT_STREAM: &str = "chat-stream";
pub const EVENT_DONE: &str = "chat-done";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamPayload {
    pub delta: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DonePayload {
    pub content: String,
    pub error: Option<String>,
}

/// 从单行 SSE 文本中提取增量内容；非内容行返回 None
pub fn extract_delta(line: &str) -> Option<String> {
    let data = line.strip_prefix("data:")?.trim();

    if data.is_empty() || data == "[DONE]" {
        return None;
    }

    let value = serde_json::from_str::<Value>(data).ok()?;

    value["choices"][0]["delta"]["content"]
        .as_str()
        .map(str::to_string)
}

pub async fn stream_chat(
    app: AppHandle,
    config: ChatConfig,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    if config.api_key.trim().is_empty() {
        return Err("未配置 API Key，请先在设置中填写".to_string());
    }

    let client = Client::new();

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let mut body = json!({
        "model": config.model,
        "stream": true,
        "messages": messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect::<Vec<_>>(),
    });

    if config.reasoning {
        body["reasoning_effort"] = Value::String("high".to_string());
        body["thinking"] = json!({ "type": "enabled" });
    }

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("请求失败: {err}"))?;

    let status = response.status();

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();

        return Err(format!("接口返回 {status}: {text}"));
    }

    let mut stream = response.bytes_stream();

    let mut buffer = Vec::new();

    let mut full = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| format!("读取响应流失败: {err}"))?;

        buffer.extend_from_slice(&chunk);

        while let Some(pos) = buffer.iter().position(|byte| *byte == b'\n') {
            let line = buffer.drain(..=pos).collect::<Vec<u8>>();

            let line = String::from_utf8_lossy(&line).trim().to_string();

            if let Some(delta) = extract_delta(&line) {
                full.push_str(&delta);

                let _ = app.emit(EVENT_STREAM, StreamPayload { delta });
            }
        }
    }

    let _ = app.emit(EVENT_DONE, DonePayload { content: full.clone(), error: None });

    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Step 1 中的测试代码原样保留
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo test chat::stream`
Expected: 5 个测试全部 PASS。

- [ ] **Step 5: 注册模块**

修改 `src-tauri/src/chat/mod.rs`：

```rust
pub mod config;
pub mod history;
pub mod stream;
```

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo check`
Expected: 编译通过。

- [ ] **Step 6: Checkpoint**

确认 `cargo test chat::stream` 全绿、`cargo check` 通过。

---

### Task 5: 命令模块（6 个 Tauri 命令 + 注册）

**Files:**
- Create: `src-tauri/src/chat/commands.rs`
- Modify: `src-tauri/src/chat/mod.rs`（追加 `pub mod commands;`）
- Modify: `src-tauri/src/lib.rs`（注册命令）

**Interfaces:**
- Produces: 命令 `open_chat_window`、`get_chat_config`、`save_chat_config`、`get_chat_history`、`clear_chat_history`、`send_chat_message`。常量 `CHAT_WINDOW_LABEL: &str = "chat"`。
- Consumes: `config::{ChatConfig, load, save}`、`history::{ChatMessage, MAX_CONTEXT_MESSAGES, append, load, clear}`、`stream::{stream_chat, DonePayload, EVENT_DONE}`。

- [ ] **Step 1: 创建命令模块**

创建 `src-tauri/src/chat/commands.rs`：

```rust
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager, Runtime, command};
use tauri_plugin_custom_window::MAIN_WINDOW_LABEL;

use super::config::{self, ChatConfig};
use super::history::{self, ChatMessage, MAX_CONTEXT_MESSAGES};
use super::stream::{self, DonePayload, EVENT_DONE};

pub const CHAT_WINDOW_LABEL: &str = "chat";

const SYSTEM_PROMPT: &str =
    "你是 BongoCat 桌面宠物里的 AI 助手。请用简洁、友好、口语化的中文回答用户的问题。";

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[command]
pub fn open_chat_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let Some(chat_window) = app.get_webview_window(CHAT_WINDOW_LABEL) else {
        return Err("聊天窗口不存在".to_string());
    };

    let _ = chat_window.show();
    let _ = chat_window.set_focus();

    // 将聊天窗口定位到猫咪主窗口右侧；越界时翻到左侧，并夹紧在屏幕内
    if let Some(main_window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if let (Ok(position), Ok(size)) = (main_window.outer_position(), main_window.outer_size()) {
            let gap = 16;

            let mut target_x = position.x + size.width as i32 + gap;
            let target_y = position.y;

            if let Ok(Some(monitor)) = chat_window.current_monitor() {
                let monitor_pos = monitor.position();
                let monitor_size = monitor.size();
                let margin = 8;

                let window_size = chat_window.outer_size().unwrap_or_default();
                let window_width = if window_size.width > 0 { window_size.width as i32 } else { 380 };
                let window_height = if window_size.height > 0 { window_size.height as i32 } else { 560 };

                let max_x = monitor_pos.x + monitor_size.width as i32 - window_width - margin;
                let min_x = monitor_pos.x + margin;

                let min_y = monitor_pos.y + margin;
                let max_y = monitor_pos.y + monitor_size.height as i32 - window_height - margin;

                if target_x > max_x {
                    target_x = position.x - window_width - gap;
                }

                let clamped_x = target_x.clamp(min_x, max_x);
                let clamped_y = target_y.clamp(min_y, max_y);

                let _ = chat_window
                    .set_position(tauri::PhysicalPosition::new(clamped_x, clamped_y));
            }
        }
    }

    Ok(())
}

#[command]
pub fn get_chat_config<R: Runtime>(app: AppHandle<R>) -> ChatConfig {
    config::load(&app)
}

#[command]
pub fn save_chat_config<R: Runtime>(app: AppHandle<R>, config: ChatConfig) -> Result<(), String> {
    config::save(&app, &config)
}

#[command]
pub fn get_chat_history<R: Runtime>(app: AppHandle<R>) -> Vec<ChatMessage> {
    history::load(&app)
}

#[command]
pub fn clear_chat_history<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    history::clear(&app)
}

#[command]
pub fn send_chat_message<R: Runtime>(app: AppHandle<R>, content: String) -> Result<(), String> {
    let content = content.trim();

    if content.is_empty() {
        return Err("消息不能为空".to_string());
    }

    let config = config::load(&app);

    if config.api_key.trim().is_empty() {
        return Err("未配置 API Key，请先在设置中填写".to_string());
    }

    let user_message = ChatMessage {
        role: "user".to_string(),
        content: content.to_string(),
        timestamp: now_timestamp(),
    };

    history::append(&app, user_message)?;

    let messages = history::load(&app);

    let context = messages[messages.len().saturating_sub(MAX_CONTEXT_MESSAGES)..].to_vec();

    let mut request_messages = vec![ChatMessage {
        role: "system".to_string(),
        content: SYSTEM_PROMPT.to_string(),
        timestamp: 0,
    }];

    request_messages.extend(context);

    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        match stream::stream_chat(app_handle.clone(), config, request_messages).await {
            Ok(content) => {
                let assistant_message = ChatMessage {
                    role: "assistant".to_string(),
                    content,
                    timestamp: now_timestamp(),
                };

                let _ = history::append(&app_handle, assistant_message);
            }
            Err(error) => {
                let _ = app_handle.emit(
                    EVENT_DONE,
                    DonePayload {
                        content: String::new(),
                        error: Some(error),
                    },
                );
            }
        }
    });

    Ok(())
}
```

- [ ] **Step 2: 注册模块与命令**

修改 `src-tauri/src/chat/mod.rs`：

```rust
pub mod commands;
pub mod config;
pub mod history;
pub mod stream;
```

修改 `src-tauri/src/lib.rs`：顶部加 `use`，invoke_handler 里注册命令：

```rust
mod chat;
mod core;
mod utils;

use chat::commands::{
    clear_chat_history, get_chat_config, get_chat_history, open_chat_window, save_chat_config,
    send_chat_message,
};
```

在 `generate_handler![...]` 中加入：

```rust
        .invoke_handler(generate_handler![
            copy_dir,
            start_device_listening,
            start_gamepad_listing,
            stop_gamepad_listing,
            open_chat_window,
            get_chat_config,
            save_chat_config,
            get_chat_history,
            clear_chat_history,
            send_chat_message
        ])
```

- [ ] **Step 3: 验证编译**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo check`
Expected: 编译通过，无 warning（若有 `unused` warning 检查 `use` 是否正确）。

- [ ] **Step 4: Checkpoint**

确认 `cargo check` 通过、`cargo test` 全绿。

---

### Task 6: 注册 chat 窗口（tauri.conf.json）

**Files:**
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Produces: 标签为 `chat` 的窗口（初始隐藏），`index.html/#/chat` 路由，默认 `380×560`。

- [ ] **Step 1: 添加窗口配置**

在 `tauri.conf.json` 的 `app.windows` 数组（`preference` 窗口之后）追加：

```json
{
  "label": "chat",
  "title": "BongoCat",
  "url": "index.html/#/chat",
  "visible": false,
  "width": 380,
  "height": 560,
  "minWidth": 320,
  "minHeight": 460,
  "skipTaskbar": true
}
```

说明：capabilities 已用 `"windows": ["*"]`，无需改动。

- [ ] **Step 2: 验证编译**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo check`
Expected: 编译通过（`generate_context!` 会校验窗口配置）。

- [ ] **Step 3: Checkpoint**

确认窗口配置合法、`cargo check` 通过。

---

### Task 7: 前端常量、类型与插件封装

**Files:**
- Modify: `src/constants/index.ts`
- Create: `src/types/chat.ts`
- Create: `src/plugins/chat.ts`

**Interfaces:**
- Produces: `LISTEN_KEY.CHAT_STREAM` / `CHAT_DONE`、`INVOKE_KEY.OPEN_CHAT_WINDOW` / `GET_CHAT_CONFIG` / `SAVE_CHAT_CONFIG` / `GET_CHAT_HISTORY` / `CLEAR_CHAT_HISTORY` / `SEND_CHAT_MESSAGE`、`WINDOW_LABEL.CHAT`。类型 `ChatMessage`、`ChatConfig`、`ChatMessageItem`。插件函数 `openChatWindow`、`getChatConfig`、`saveChatConfig(config)`、`getChatHistory`、`clearChatHistory`、`sendChatMessage(content)`。

- [ ] **Step 1: 扩展常量**

在 `src/constants/index.ts` 的 `LISTEN_KEY` 中追加：

```ts
  CHAT_STREAM: 'chat-stream',
  CHAT_DONE: 'chat-done',
```

在 `INVOKE_KEY` 中追加：

```ts
  OPEN_CHAT_WINDOW: 'open_chat_window',
  GET_CHAT_CONFIG: 'get_chat_config',
  SAVE_CHAT_CONFIG: 'save_chat_config',
  GET_CHAT_HISTORY: 'get_chat_history',
  CLEAR_CHAT_HISTORY: 'clear_chat_history',
  SEND_CHAT_MESSAGE: 'send_chat_message',
```

在 `WINDOW_LABEL` 中追加：

```ts
  CHAT: 'chat',
```

- [ ] **Step 2: 创建类型**

创建 `src/types/chat.ts`：

```ts
export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  timestamp: number
}

export interface ChatConfig {
  apiKey: string
  baseUrl: string
  model: string
  reasoning: boolean
}

export interface ChatMessageItem extends ChatMessage {
  id: string
  streaming: boolean
}
```

- [ ] **Step 3: 创建插件封装**

创建 `src/plugins/chat.ts`（仿照现有 `src/plugins/window.ts` 风格）：

```ts
import { invoke } from '@tauri-apps/api/core'

import type { ChatConfig, ChatMessage } from '@/types/chat'

import { INVOKE_KEY } from '../constants'

export function openChatWindow() {
  return invoke<void>(INVOKE_KEY.OPEN_CHAT_WINDOW)
}

export function getChatConfig() {
  return invoke<ChatConfig>(INVOKE_KEY.GET_CHAT_CONFIG)
}

export function saveChatConfig(config: ChatConfig) {
  return invoke<void>(INVOKE_KEY.SAVE_CHAT_CONFIG, { config })
}

export function getChatHistory() {
  return invoke<ChatMessage[]>(INVOKE_KEY.GET_CHAT_HISTORY)
}

export function clearChatHistory() {
  return invoke<void>(INVOKE_KEY.CLEAR_CHAT_HISTORY)
}

export function sendChatMessage(content: string) {
  return invoke<void>(INVOKE_KEY.SEND_CHAT_MESSAGE, { content })
}
```

- [ ] **Step 4: Checkpoint**

Run: `cd "D:\CL\BongoCat-master" && pnpm lint`
Expected: 无新增错误。

---

### Task 8: 聊天状态 store

**Files:**
- Create: `src/stores/chat.ts`

**Interfaces:**
- Produces: `useChatStore()`，含 `messages: Ref<ChatMessageItem[]>`、`config: Ref<ChatConfig>`、`streaming: Ref<boolean>`、`error: Ref<string>`、`configVisible: Ref<boolean>`，方法 `init()`、`appendUser(content)`、`clear()`、`saveConfig(config)`。
- Consumes: `src/plugins/chat.ts` 的 `getChatConfig` / `getChatHistory` / `clearChatHistory` / `saveChatConfig`。

- [ ] **Step 1: 创建 store**

创建 `src/stores/chat.ts`：

```ts
import { nanoid } from 'nanoid'
import { defineStore } from 'pinia'
import { ref } from 'vue'

import { clearChatHistory, getChatConfig, getChatHistory, saveChatConfig } from '@/plugins/chat'
import type { ChatConfig, ChatMessageItem } from '@/types/chat'

export const useChatStore = defineStore('chat', () => {
  const messages = ref<ChatMessageItem[]>([])
  const config = ref<ChatConfig>({ apiKey: '', baseUrl: '', model: '', reasoning: true })
  const streaming = ref(false)
  const error = ref('')
  const configVisible = ref(false)

  const init = async () => {
    const [cfg, history] = await Promise.all([getChatConfig(), getChatHistory()])

    config.value = cfg

    messages.value = history.map(message => ({
      ...message,
      id: nanoid(),
      streaming: false,
    }))
  }

  const appendUser = (content: string) => {
    messages.value.push({
      role: 'user',
      content,
      timestamp: Math.floor(Date.now() / 1000),
      id: nanoid(),
      streaming: false,
    })
  }

  const clear = async () => {
    await clearChatHistory()

    messages.value = []
  }

  const saveConfig = async (nextConfig: ChatConfig) => {
    await saveChatConfig(nextConfig)

    config.value = nextConfig
  }

  return {
    messages,
    config,
    streaming,
    error,
    configVisible,
    init,
    appendUser,
    clear,
    saveConfig,
  }
}, {
  tauri: {
    save: false,
    sync: false,
  },
})
```

说明：`tauri: { save: false, sync: false }` 让 tauri-store 不持久化本 store —— 聊天记录由 Rust 端 `chat-history.json` 负责，避免双写冲突。

- [ ] **Step 2: Checkpoint**

Run: `cd "D:\CL\BongoCat-master" && pnpm lint`
Expected: 无新增错误。

---

### Task 9: 流式编排 composable

**Files:**
- Create: `src/composables/useChat.ts`

**Interfaces:**
- Produces: `useChat()`，返回 `{ send(content: string): Promise<void> }`。
- Consumes: `useChatStore`、`LISTEN_KEY.CHAT_STREAM`/`CHAT_DONE`、`sendChatMessage`、`useTauriListen`。

- [ ] **Step 1: 创建 composable**

创建 `src/composables/useChat.ts`：

```ts
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { nanoid } from 'nanoid'

import { LISTEN_KEY } from '@/constants'
import { sendChatMessage } from '@/plugins/chat'
import { useChatStore } from '@/stores/chat'
import { useTauriListen } from './useTauriListen'

const appWindow = getCurrentWebviewWindow()

export function useChat() {
  const chatStore = useChatStore()

  useTauriListen<string>(LISTEN_KEY.CHAT_STREAM, ({ payload }) => {
    if (appWindow.label !== 'chat') return

    const current = chatStore.messages[chatStore.messages.length - 1]

    if (current?.role !== 'assistant' || !current.streaming) {
      chatStore.messages.push({
        role: 'assistant',
        content: '',
        timestamp: Math.floor(Date.now() / 1000),
        id: nanoid(),
        streaming: true,
      })
    }

    const last = chatStore.messages[chatStore.messages.length - 1]!

    last.content += payload
  })

  useTauriListen<{ content: string, error?: string }>(LISTEN_KEY.CHAT_DONE, ({ payload }) => {
    if (appWindow.label !== 'chat') return

    const current = chatStore.messages[chatStore.messages.length - 1]

    if (current?.role !== 'assistant' || !current.streaming) {
      chatStore.messages.push({
        role: 'assistant',
        content: payload.content,
        timestamp: Math.floor(Date.now() / 1000),
        id: nanoid(),
        streaming: false,
      })
    } else {
      current.content = payload.content
      current.streaming = false
    }

    chatStore.error = payload.error ?? ''

    chatStore.streaming = false
  })

  const send = async (content: string) => {
    const text = content.trim()

    if (!text || chatStore.streaming) return

    chatStore.error = ''
    chatStore.appendUser(text)
    chatStore.streaming = true

    try {
      await sendChatMessage(text)
    } catch (err) {
      chatStore.streaming = false
      chatStore.error = String(err)

      const last = chatStore.messages[chatStore.messages.length - 1]

      if (last?.role === 'user' && !last.streaming) {
        chatStore.messages.pop()
      }
    }
  }

  return {
    send,
  }
}
```

- [ ] **Step 2: Checkpoint**

Run: `cd "D:\CL\BongoCat-master" && pnpm lint`
Expected: 无新增错误。

---

### Task 10: 聊天页面 UI（微信风格）+ 路由

**Files:**
- Create: `src/pages/chat/index.vue`
- Modify: `src/router/index.ts`

**Interfaces:**
- Consumes: `useChat().send`、`useChatStore`（messages/config/streaming/error/configVisible/init/clear/saveConfig）、`useGeneralStore().appearance.isDark`、`useI18n`、`VueMarkdown`。

- [ ] **Step 1: 创建页面**

创建 `src/pages/chat/index.vue`：

```vue
<script setup lang="ts">
import VueMarkdown from 'vue-markdown-render'
import { Button, Drawer, Input, InputPassword, message, Popconfirm, Switch } from 'antdv-next'
import { nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { useChat } from '@/composables/useChat'
import { useChatStore } from '@/stores/chat'
import { useGeneralStore } from '@/stores/general'

const chatStore = useChatStore()
const generalStore = useGeneralStore()
const { t } = useI18n()
const { send } = useChat()

const draft = ref('')
const scrollContainer = ref<HTMLElement>()
const configForm = ref({
  apiKey: '',
  baseUrl: '',
  model: '',
  reasoning: true,
})

const scrollToBottom = () => {
  nextTick(() => {
    const el = scrollContainer.value

    if (el) el.scrollTop = el.scrollHeight
  })
}

const handleSend = async () => {
  const content = draft.value.trim()

  if (!content || chatStore.streaming) return

  draft.value = ''
  scrollToBottom()
  await send(content)
}

const handleClear = async () => {
  await chatStore.clear()
}

const handleSaveConfig = async () => {
  await chatStore.saveConfig({
    apiKey: configForm.value.apiKey.trim(),
    baseUrl: configForm.value.baseUrl.trim(),
    model: configForm.value.model.trim(),
    reasoning: configForm.value.reasoning,
  })

  chatStore.configVisible = false
  message.success(t('pages.chat.saved'))
}

watch(() => chatStore.configVisible, (visible) => {
  if (!visible) return

  configForm.value = {
    apiKey: chatStore.config.apiKey,
    baseUrl: chatStore.config.baseUrl,
    model: chatStore.config.model,
    reasoning: chatStore.config.reasoning,
  }
})

watch(() => chatStore.messages.length, scrollToBottom)

watch(() => generalStore.appearance.isDark, (value) => {
  document.documentElement.classList.toggle('dark', value)
}, { immediate: true })

onMounted(() => {
  chatStore.init()
})
</script>

<template>
  <div class="flex h-screen flex-col bg-[#ededed] dark:bg-[#111014]">
    <!-- 顶栏 -->
    <header class="flex h-12 shrink-0 items-center justify-between border-b border-black/10 bg-[#f7f7f7] px-3 dark:border-white/10 dark:bg-[#1f1f1f]">
      <div class="flex min-w-0 items-center gap-2">
        <span class="truncate text-sm font-semibold text-neutral-800 dark:text-white">{{ $t('pages.chat.title') }}</span>
        <span class="shrink-0 rounded bg-[#07c160]/10 px-1.5 py-0.5 text-xs text-[#07c160]">{{ chatStore.config.model || $t('pages.chat.modelUnset') }}</span>
      </div>
      <div class="flex shrink-0 items-center gap-1">
        <Popconfirm
          :title="$t('pages.chat.clearConfirm')"
          ok-text="确定"
          cancel-text="取消"
          @confirm="handleClear"
        >
          <Button type="text" size="small">
            {{ $t('pages.chat.clear') }}
          </Button>
        </Popconfirm>
        <Button type="text" size="small" @click="chatStore.configVisible = true">
          {{ $t('pages.chat.settings') }}
        </Button>
      </div>
    </header>

    <!-- 消息列表 -->
    <div ref="scrollContainer" class="flex-1 overflow-y-auto px-3 py-4">
      <div
        v-if="chatStore.messages.length === 0"
        class="flex h-full flex-col items-center justify-center gap-2 text-sm text-neutral-400"
      >
        <div class="text-5xl">🐱</div>
        <div>{{ $t('pages.chat.empty') }}</div>
      </div>

      <div
        v-for="item in chatStore.messages"
        :key="item.id"
        class="mb-4 flex items-start"
        :class="item.role === 'user' ? 'justify-end' : 'justify-start'"
      >
        <template v-if="item.role === 'assistant'">
          <div class="mr-2 flex size-8 shrink-0 items-center justify-center rounded-lg bg-[#07c160] text-lg text-white">🐱</div>
          <div class="max-w-[70%] rounded-xl rounded-tl-sm border border-black/5 bg-white px-3 py-2 text-sm leading-relaxed text-neutral-800 shadow-sm dark:border-white/10 dark:bg-[#26262b] dark:text-neutral-100">
            <VueMarkdown v-if="item.content" :source="item.content" />
            <div v-else-if="item.streaming" class="flex items-center gap-1 py-1 text-[#07c160]">
              <span class="typing-dot" />
              <span class="typing-dot" />
              <span class="typing-dot" />
            </div>
          </div>
        </template>
        <template v-else>
          <div class="max-w-[70%] whitespace-pre-wrap break-words rounded-xl rounded-tr-sm bg-[#07c160] px-3 py-2 text-sm leading-relaxed text-white shadow-sm">
            {{ item.content }}
          </div>
          <div class="ml-2 flex size-8 shrink-0 items-center justify-center rounded-lg bg-[#07c160] text-lg text-white">😺</div>
        </template>
      </div>

      <div v-if="chatStore.error" class="mb-2 text-center text-xs text-red-500">
        {{ chatStore.error }}
      </div>
    </div>

    <!-- 输入区 -->
    <footer class="shrink-0 border-t border-black/10 bg-[#f7f7f7] p-2 dark:border-white/10 dark:bg-[#1f1f1f]">
      <textarea
        v-model="draft"
        class="h-16 w-full resize-none rounded-lg border border-black/10 bg-white px-3 py-2 text-sm text-neutral-800 outline-none transition-colors dark:border-white/10 dark:bg-black/30 dark:text-white"
        :placeholder="$t('pages.chat.placeholder')"
        @keydown.enter.exact.prevent="handleSend"
      />
      <div class="mt-2 flex items-center justify-end gap-2">
        <span class="text-xs text-neutral-400">{{ $t('pages.chat.enterHint') }}</span>
        <Button
          type="primary"
          class="!bg-[#07c160]"
          :loading="chatStore.streaming"
          :disabled="chatStore.streaming"
          @click="handleSend"
        >
          {{ $t('pages.chat.send') }}
        </Button>
      </div>
    </footer>

    <!-- 设置抽屉 -->
    <Drawer
      v-model:open="chatStore.configVisible"
      :title="$t('pages.chat.settingsTitle')"
      width="320"
      :footer="null"
    >
      <div class="space-y-5">
        <div>
          <div class="mb-1 text-xs text-neutral-500 dark:text-neutral-400">{{ $t('pages.chat.apiKey') }}</div>
          <InputPassword v-model:value="configForm.apiKey" placeholder="sk-..." />
        </div>
        <div>
          <div class="mb-1 text-xs text-neutral-500 dark:text-neutral-400">{{ $t('pages.chat.baseUrl') }}</div>
          <Input v-model:value="configForm.baseUrl" placeholder="https://api.deepseek.com" />
        </div>
        <div>
          <div class="mb-1 text-xs text-neutral-500 dark:text-neutral-400">{{ $t('pages.chat.model') }}</div>
          <Input v-model:value="configForm.model" placeholder="deepseek-v4-pro" />
        </div>
        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm text-neutral-800 dark:text-white">{{ $t('pages.chat.reasoning') }}</div>
            <div class="text-xs text-neutral-400">{{ $t('pages.chat.reasoningHint') }}</div>
          </div>
          <Switch v-model:checked="configForm.reasoning" />
        </div>
        <div class="flex justify-end gap-2 pt-2">
          <Button @click="chatStore.configVisible = false">
            {{ $t('pages.chat.cancel') }}
          </Button>
          <Button type="primary" class="!bg-[#07c160]" @click="handleSaveConfig">
            {{ $t('pages.chat.save') }}
          </Button>
        </div>
      </div>
    </Drawer>
  </div>
</template>

<style scoped>
.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.4;
  animation: typing 1.2s infinite;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes typing {
  0%, 100% {
    opacity: 0.4;
    transform: translateY(0);
  }
  50% {
    opacity: 1;
    transform: translateY(-3px);
  }
}

:deep(.markdown-body p) {
  margin: 0;
}
</style>
```

说明：`Popconfirm` 的 `ok-text`/`cancel-text` 用固定中文，避免为这两个按钮词再加一组 i18n 键（若希望国际化，可后续加键）。

- [ ] **Step 2: 添加路由**

修改 `src/router/index.ts`：

```ts
import Chat from '../pages/chat/index.vue'
```

在路由数组中追加：

```ts
  {
    path: '/chat',
    component: Chat,
  },
```

- [ ] **Step 3: Checkpoint**

Run: `cd "D:\CL\BongoCat-master" && pnpm lint`
Expected: 无新增错误（`pages.chat.*` 未定义的键仅在运行时回显，不影响编译）。

---

### Task 11: 主窗口双击打开聊天

**Files:**
- Modify: `src/pages/main/index.vue`

**Interfaces:**
- Consumes: `@/plugins/chat` 的 `openChatWindow`。

- [ ] **Step 1: 引入插件**

在 `src/pages/main/index.vue` 的 import 区块（`import { hideWindow, setAlwaysOnTop, setTaskbarVisibility, showWindow } from '@/plugins/window'` 附近）加入：

```ts
import { openChatWindow } from '@/plugins/chat'
```

- [ ] **Step 2: 双击判定逻辑**

在 `<script setup>` 中（`function handleMouseDown` 之前）添加常量与状态：

```ts
const DOUBLE_CLICK_MS = 300
const DOUBLE_CLICK_DISTANCE = 5
const DRAG_DELAY_MS = 200

let lastClick = { time: 0, x: 0, y: 0 }
let dragTimer: ReturnType<typeof setTimeout> | undefined
```

将 `handleMouseDown` 整体替换为：

```ts
function handleMouseDown(event: MouseEvent) {
  if (event.button === 2) {
    // 右键保持原行为：立即进入原生拖拽，以支持 Shift+右键 缩放时跨窗口边界的追踪
    appWindow.startDragging()

    return
  }

  if (event.button !== 0) return

  const now = Date.now()
  const isDoubleClick = now - lastClick.time < DOUBLE_CLICK_MS
    && Math.hypot(event.clientX - lastClick.x, event.clientY - lastClick.y) < DOUBLE_CLICK_DISTANCE

  lastClick = { time: now, x: event.clientX, y: event.clientY }

  if (dragTimer) {
    clearTimeout(dragTimer)

    dragTimer = void 0
  }

  if (isDoubleClick) {
    openChatWindow()

    lastClick.time = 0

    return
  }

  dragTimer = setTimeout(() => {
    appWindow.startDragging()
  }, DRAG_DELAY_MS)
}
```

说明：左键单击延迟 200ms 再拖拽，以区分单击拖拽与双击聊天；双击时取消拖拽并打开聊天窗口。右键仍立即拖拽，保证 Shift+右键缩放功能不变；右键菜单（contextmenu）不受影响。

- [ ] **Step 3: Checkpoint**

Run: `cd "D:\CL\BongoCat-master" && pnpm lint`
Expected: 无新增错误。

---

### Task 12: 五语言 i18n 文案

**Files:**
- Modify: `src/locales/zh-CN.json`
- Modify: `src/locales/zh-TW.json`
- Modify: `src/locales/en-US.json`
- Modify: `src/locales/vi-VN.json`
- Modify: `src/locales/pt-BR.json`

**Interfaces:**
- Produces: 各语言文件 `pages.chat` 节点，键与 Task 10 模板中 `$t` 引用一致：`title`、`modelUnset`、`clear`、`clearConfirm`、`settings`、`settingsTitle`、`apiKey`、`baseUrl`、`model`、`reasoning`、`reasoningHint`、`cancel`、`save`、`saved`、`empty`、`placeholder`、`enterHint`、`send`。

- [ ] **Step 1: 在 zh-CN.json 的 `pages` 节点追加**

在 `zh-CN.json` 的 `pages` 对象内（`main` 节点之后）添加：

```json
    "chat": {
      "title": "猫咪助手",
      "modelUnset": "未设置模型",
      "clear": "清空",
      "clearConfirm": "确定要清空当前对话吗？",
      "settings": "设置",
      "settingsTitle": "模型设置",
      "apiKey": "API Key",
      "baseUrl": "Base URL",
      "model": "模型",
      "reasoning": "推理模式",
      "reasoningHint": "启用后附带推理参数（deepseek-v4-pro 等推理模型使用）",
      "cancel": "取消",
      "save": "保存",
      "saved": "设置已保存",
      "empty": "你好呀，我是猫咪助手，有什么可以帮你？",
      "placeholder": "输入消息…",
      "enterHint": "Enter 发送 / Shift+Enter 换行",
      "send": "发送"
    }
```

- [ ] **Step 2: 在 en-US.json 的 `pages` 节点追加**

```json
    "chat": {
      "title": "Cat Assistant",
      "modelUnset": "No model set",
      "clear": "Clear",
      "clearConfirm": "Clear current conversation?",
      "settings": "Settings",
      "settingsTitle": "Model Settings",
      "apiKey": "API Key",
      "baseUrl": "Base URL",
      "model": "Model",
      "reasoning": "Reasoning",
      "reasoningHint": "Enables reasoning parameters (used by deepseek-v4-pro and other reasoning models)",
      "cancel": "Cancel",
      "save": "Save",
      "saved": "Settings saved",
      "empty": "Hi! I'm your cat assistant. How can I help?",
      "placeholder": "Type a message…",
      "enterHint": "Enter to send / Shift+Enter for a new line",
      "send": "Send"
    }
```

- [ ] **Step 3: 在 zh-TW.json 的 `pages` 节点追加**

```json
    "chat": {
      "title": "貓咪助手",
      "modelUnset": "未設定模型",
      "clear": "清空",
      "clearConfirm": "確定要清空目前對話嗎？",
      "settings": "設定",
      "settingsTitle": "模型設定",
      "apiKey": "API Key",
      "baseUrl": "Base URL",
      "model": "模型",
      "reasoning": "推理模式",
      "reasoningHint": "啟用後會附加推理參數（deepseek-v4-pro 等推理模型使用）",
      "cancel": "取消",
      "save": "儲存",
      "saved": "設定已儲存",
      "empty": "你好呀，我是貓咪助手，有什麼可以幫你？",
      "placeholder": "輸入訊息…",
      "enterHint": "Enter 傳送 / Shift+Enter 換行",
      "send": "傳送"
    }
```

- [ ] **Step 4: 在 vi-VN.json 的 `pages` 节点追加**

```json
    "chat": {
      "title": "Trợ lý Mèo",
      "modelUnset": "Chưa đặt model",
      "clear": "Xóa",
      "clearConfirm": "Xóa cuộc trò chuyện hiện tại?",
      "settings": "Cài đặt",
      "settingsTitle": "Cài đặt Model",
      "apiKey": "API Key",
      "baseUrl": "Base URL",
      "model": "Model",
      "reasoning": "Chế độ suy luận",
      "reasoningHint": "Bật sẽ gửi kèm tham số suy luận (dùng cho deepseek-v4-pro và các model suy luận khác)",
      "cancel": "Hủy",
      "save": "Lưu",
      "saved": "Đã lưu cài đặt",
      "empty": "Chào bạn, mình là trợ lý Mèo, có gì giúp được bạn?",
      "placeholder": "Nhập tin nhắn…",
      "enterHint": "Enter để gửi / Shift+Enter để xuống dòng",
      "send": "Gửi"
    }
```

- [ ] **Step 5: 在 pt-BR.json 的 `pages` 节点追加**

```json
    "chat": {
      "title": "Assistente do Gato",
      "modelUnset": "Nenhum modelo definido",
      "clear": "Limpar",
      "clearConfirm": "Limpar a conversa atual?",
      "settings": "Configurações",
      "settingsTitle": "Configurações do Modelo",
      "apiKey": "API Key",
      "baseUrl": "Base URL",
      "model": "Modelo",
      "reasoning": "Modo de raciocínio",
      "reasoningHint": "Habilita parâmetros de raciocínio (usado por deepseek-v4-pro e outros modelos de raciocínio)",
      "cancel": "Cancelar",
      "save": "Salvar",
      "saved": "Configurações salvas",
      "empty": "Olá! Sou o assistente do gato. Como posso ajudar?",
      "placeholder": "Digite uma mensagem…",
      "enterHint": "Enter para enviar / Shift+Enter para nova linha",
      "send": "Enviar"
    }
```

- [ ] **Step 6: Checkpoint**

Run: `cd "D:\CL\BongoCat-master" && pnpm lint`
Expected: 无新增错误。

---

### Task 13: 端到端验证

**Files:**
- 无需新增/修改。

- [ ] **Step 1: Rust 全量测试**

Run: `cd "D:\CL\BongoCat-master\src-tauri" && cargo test`
Expected: 全部测试 PASS（含 chat::config / chat::history / chat::stream）。

- [ ] **Step 2: 前端 lint 与构建**

Run: `cd "D:\CL\BongoCat-master" && pnpm lint && pnpm build`
Expected: lint 无错误；`vite build` 成功。

- [ ] **Step 3: 手动端到端验证（tauri dev）**

Run: `cd "D:\CL\BongoCat-master" && pnpm tauri dev`

按以下清单验证：
1. 双击桌面猫咪 → 聊天窗口在猫咪右侧弹出，位置不越屏。
2. 设置抽屉填入真实 API Key / base_url / model，保存后提示「设置已保存」。
3. 输入消息发送 → 出现左侧猫咪头像气泡，内容流式逐字显示（打字动画）。
4. 发送过程中发送按钮禁用；完成后气泡停止，输入框恢复。
5. 发送空消息 / 未配置 key → 出现错误提示。
6. 重启应用 → 重新打开聊天窗口，历史对话仍存在。
7. 点击清空 → 二次确认后对话清空。
8. 确认 `%APPDATA%\com.ayangweb.BongoCat\.env` 与 `chat-history.json` 已生成且内容正确。
9. 主窗口：单击拖拽仍正常（有约 200ms 延迟）；右键菜单、Shift+右键缩放不受影响。
10. 切换暗色主题后聊天窗口配色跟随（`html.dark` 生效）。

- [ ] **Step 4: 收尾 Checkpoint**

确认全部验证通过后，向用户汇报：新增文件清单、命令清单、.env 配置位置与格式、如何切换其他 OpenAI 兼容模型、注意事项（key 明文存放、推理开关作用）。
