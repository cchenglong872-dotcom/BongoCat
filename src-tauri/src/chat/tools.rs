use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use tauri::{AppHandle, Manager, Runtime};

use super::config::ChatConfig;

/// 打包后的工具目录（位于应用资源目录下的 tools 文件夹）
fn tools_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .resource_dir()
        .map(|dir| dir.join("tools"))
        .map_err(|err| format!("无法获取资源目录: {err}"))
}

/// 供请求体使用的工具定义（OpenAI function calling 格式）
pub fn tools_definition() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "query_weather",
                "description": "查询城市的实时天气和未来 3 天天气预报。用户未指明城市时，省略 city 参数，将自动查询设置中的默认省市。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "city": { "type": "string", "description": "城市名（可选），例如：西安、北京。留空时使用默认省市" }
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "query_ticket",
                "description": "查询 12306 火车直达余票信息",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "description": "出发日期，格式 YYYY-MM-DD，不能早于当天" },
                        "from_station": { "type": "string", "description": "出发站，例如：北京" },
                        "to_station": { "type": "string", "description": "到达站，例如：上海" }
                    },
                    "required": ["date", "from_station", "to_station"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "schedule_shutdown",
                "description": "设置定时关机，或取消已设置的定时关机",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "minutes": { "type": "integer", "description": "延迟关机的分钟数，仅在 cancel 为 false 时需要" },
                        "cancel": { "type": "boolean", "description": "是否取消已设置的定时关机" }
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "query_weibo_hotsearch",
                "description": "查询微博实时热搜榜前 N 条（无需账号）",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "返回条数，默认 10" }
                    }
                }
            }
        }
    ])
}

/// 执行打包后的工具可执行文件，返回 stdout；失败返回 stderr 或通用错误
fn run_tool(
    exe_name: &str,
    args: &[&str],
    envs: &[(&str, &str)],
    tools: &Path,
) -> Result<String, String> {
    let mut exe_path = tools.join(exe_name);

    // Windows 下 PyInstaller 产物带 .exe 后缀
    #[cfg(target_os = "windows")]
    exe_path.set_extension("exe");

    let mut command = Command::new(&exe_path);

    // Windows 下隐藏控制台子进程的黑窗口：PyInstaller 产物是 console 子系统，
    // 被 GUI 进程启动时 Windows 会默认弹出新的控制台窗口。用 CREATE_NO_WINDOW
    // 保留控制台（stdout 管道照常捕获结果）但不显示窗口。
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .args(args)
        .current_dir(tools)
        .env("PYTHONIOENCODING", "utf-8");

    for (key, value) in envs {
        command.env(key, value);
    }

    let output = command
        .output()
        .map_err(|err| format!("执行 {exe_name} 失败: {err}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        Err(if stderr.is_empty() { format!("{exe_name} 执行失败") } else { stderr })
    }
}

/// 确保车票查询所需的站名表存在于可写目录，返回该目录（同时作为脚本缓存目录）
fn ensure_ticket_data<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let cache_dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("无法获取配置目录: {err}"))?;

    let target = cache_dir.join("stations.json");

    // 首次查询时，把资源目录里的站名表复制到可写配置目录，保证离线可用
    if !target.exists() {
        let source = tools_dir(app)?.join("stations.json");

        if source.exists() {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            std::fs::copy(&source, &target).map_err(|err| format!("复制站名表失败: {err}"))?;
        }
    }

    Ok(cache_dir)
}

/// 由配置的省市拼出默认查询位置；两者皆空时返回空串
fn default_location(config: &ChatConfig) -> String {
    let province = config.weather_province.trim();
    let city = config.weather_city.trim();

    match (province.is_empty(), city.is_empty()) {
        (true, true) => String::new(),
        (false, false) => format!("{province} {city}"),
        (_, _) => province.to_string() + city,
    }
}

/// 执行指定工具，返回工具结果文本
pub fn execute_tool<R: Runtime>(
    app: &AppHandle<R>,
    name: &str,
    arguments: &Value,
    config: &ChatConfig,
) -> Result<String, String> {
    let tools = tools_dir(app)?;

    match name {
        "query_weather" => {
            let api_key = config.weather_api_key.trim();

            if api_key.is_empty() {
                return Err(
                    "天气查询失败：尚未配置和风天气 API Key。请直接提醒用户到聊天设置中填写“和风天气 API Key”，不要编造天气数据。"
                        .to_string(),
                );
            }

            let mut city = arguments
                .get("city")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default()
                .to_string();

            if city.is_empty() {
                city = default_location(config);
            }

            if city.is_empty() {
                return Err(
                    "天气查询失败：未提供城市且尚未设置默认省市。请提醒用户在聊天设置中填写省份和城市。".to_string(),
                );
            }

            let envs = [("X-QW-Api-Key", api_key)];

            run_tool("weather_tool", &[city.as_str()], &envs, &tools)
        }
        "query_ticket" => {
            let date = arguments
                .get("date")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少出发日期参数".to_string())?;
            let from_station = arguments
                .get("from_station")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少出发站参数".to_string())?;
            let to_station = arguments
                .get("to_station")
                .and_then(Value::as_str)
                .ok_or_else(|| "缺少到达站参数".to_string())?;

            let cache_dir = ensure_ticket_data(app)?;
            let cache_dir = cache_dir.to_string_lossy().to_string();
            let envs = [("BONGO_CACHE_DIR", cache_dir.as_str())];

            run_tool(
                "ticket_tool",
                &[
                    "get-tickets",
                    "--date",
                    date,
                    "--from_station",
                    from_station,
                    "--to_station",
                    to_station,
                    "--format",
                    "text",
                ],
                &envs,
                &tools,
            )
        }
        "schedule_shutdown" => {
            let cancel = arguments.get("cancel").and_then(Value::as_bool).unwrap_or(false);

            if cancel {
                return run_tool("shutdown_tool", &["cancel"], &[], &tools);
            }

            let minutes = arguments
                .get("minutes")
                .and_then(Value::as_i64)
                .ok_or_else(|| "缺少关机延迟分钟数参数".to_string())?;
            let seconds = (minutes * 60).to_string();

            run_tool("shutdown_tool", &[seconds.as_str()], &[], &tools)
        }
        "query_weibo_hotsearch" => {
            let limit = arguments
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(10)
                .max(1);
            let limit = limit.to_string();

            run_tool(
                "weibo_hotsearch",
                &["--limit", limit.as_str(), "--format", "text"],
                &[],
                &tools,
            )
        }
        _ => Err(format!("未知工具: {name}")),
    }
}
