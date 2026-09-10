use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;
use tauri::{AppHandle, Emitter, Manager, Runtime, command};
use tauri_plugin_custom_window::MAIN_WINDOW_LABEL;

use super::avatar;
use super::config::{self, ChatConfig};
use super::history::{self, ChatMessage, MAX_CONTEXT_MESSAGES};
use super::stream::{self, DonePayload, EVENT_DONE};

pub const CHAT_WINDOW_LABEL: &str = "chat";

fn system_prompt(config: &ChatConfig) -> String {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let mut prompt = format!(
        "你是 BongoCat 桌面宠物里的 AI 助手。请用简洁、友好、口语化的中文回答用户的问题。\n当前日期是 {today}（北京时间）。当用户提到相对日期（今天、明天、后天、下周等）时，请据此推算具体日期。",
    );

    let province = config.weather_province.trim();
    let city = config.weather_city.trim();

    if !province.is_empty() || !city.is_empty() {
        let region = match (province.is_empty(), city.is_empty()) {
            (false, false) => format!("{province} {city}"),
            _ => format!("{province}{city}"),
        };

        prompt.push_str(&format!(
            "\n用户的默认省市是 {region}。当用户询问天气但没有指明城市时，请调用天气工具并省略 city 参数，默认查询该地区。",
        ));
    }

    prompt
}

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

    // 先取消最小化，再显示并聚焦，确保最小化后仍能重新弹出聊天窗口
    let _ = chat_window.unminimize();
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

                // 窗口可能比显示器还大，此时 max < min；用 max.max(min) 兜底避免 clamp panic
                let clamped_x = target_x.clamp(min_x, max_x.max(min_x));
                let clamped_y = target_y.clamp(min_y, max_y.max(min_y));

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
pub fn save_chat_avatar<R: Runtime>(app: AppHandle<R>, source_path: String) -> Result<String, String> {
    let path = avatar::save(&app, &source_path)?;
    let mut config = config::load(&app);

    config.user_avatar_path = Some(path.clone());

    config::save(&app, &config)?;

    Ok(path)
}

#[command]
pub fn clear_chat_avatar<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    avatar::clear(&app)?;
    let mut config = config::load(&app);
    config.user_avatar_path = None;
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
        content: system_prompt(&config),
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

/// 每日自动播报的最后成功日期标记文件名（存于应用配置目录）
const DAILY_BRIEF_FILE: &str = "daily-brief-date";

/// 自动播报的触发消息：引导模型调用天气与微博热搜工具
const DAILY_BRIEF_TRIGGER: &str = "\
现在为你做一次每日定时播报，请依次完成两件事：
1. 调用天气工具查询默认省市的今日实时天气与未来几天预报，并据此用通俗的话提醒今天出行、穿衣、是否带伞等注意事项；
2. 调用微博热搜工具获取此刻的热搜榜前 10 条，挑选其中 3 到 5 条值得关注的热点简要介绍。

请用简体中文、热情友好的语气，按「今日天气与提醒」和「此刻热搜」两部分输出。
若天气工具因未配置和风天气 API Key 或未设置默认省市而失败，请如实提醒我先在聊天设置中补齐，不要编造天气数据。";

fn brief_marker_path<R: Runtime>(app: &AppHandle<R>) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(DAILY_BRIEF_FILE))
        .map_err(|err| format!("无法获取应用配置目录: {err}"))
}

fn has_briefed_today<R: Runtime>(app: &AppHandle<R>) -> bool {
    let today = Local::now().format("%Y-%m-%d").to_string();

    brief_marker_path(app)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|content| content.trim() == today)
        .unwrap_or(false)
}

fn mark_briefed_today<R: Runtime>(app: &AppHandle<R>) {
    let Ok(path) = brief_marker_path(app) else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let _ = fs::write(path, Local::now().format("%Y-%m-%d").to_string());
}

/// 程序启动后执行的每日播报：同一天只会成功触发一次
pub async fn run_daily_brief<R: Runtime>(app: AppHandle<R>) {
    if has_briefed_today(&app) {
        return;
    }

    let config = config::load(&app);

    // 尚未配置模型 API Key 时无法生成播报，静默跳过且不写日期，等配置后下次启动生效
    if config.api_key.trim().is_empty() {
        return;
    }

    let _ = open_chat_window(app.clone());

    let request_messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt(&config),
            timestamp: 0,
        },
        ChatMessage {
            role: "user".to_string(),
            content: DAILY_BRIEF_TRIGGER.to_string(),
            timestamp: now_timestamp(),
        },
    ];

    let result = stream::stream_chat(app.clone(), config, request_messages).await;

    if result.map_or(false, |content| !content.trim().is_empty()) {
        mark_briefed_today(&app);
    }
}
