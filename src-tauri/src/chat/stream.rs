use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Runtime};

use super::config::ChatConfig;
use super::history::ChatMessage;
use super::tools::{execute_tool, tools_definition};

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

fn messages_to_values(messages: &[ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| json!({ "role": &message.role, "content": &message.content }))
        .collect()
}

fn build_body(config: &ChatConfig, messages: &[Value], tools: Option<&Value>, stream: bool) -> Value {
    let mut body = json!({
        "model": config.model,
        "stream": stream,
        "messages": messages,
    });

    if let Some(tools) = tools {
        body["tools"] = tools.clone();
    }

    if config.reasoning {
        body["reasoning_effort"] = Value::String("high".to_string());
        body["thinking"] = json!({ "type": "enabled" });
    }

    body
}

async fn send_request(
    client: &Client,
    url: &str,
    config: &ChatConfig,
    body: &Value,
) -> Result<reqwest::Response, String> {
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(body)
        .send()
        .await
        .map_err(|err| format!("请求失败: {err}"))?;

    let status = response.status();

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();

        return Err(format!("接口返回 {status}: {text}"));
    }

    Ok(response)
}

async fn stream_request<R: Runtime>(
    app: &AppHandle<R>,
    client: &Client,
    url: &str,
    config: &ChatConfig,
    messages: &[Value],
    tools: Option<&Value>,
) -> Result<String, String> {
    let body = build_body(config, messages, tools, true);

    let response = send_request(client, url, config, &body).await?;

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

pub async fn stream_chat<R: Runtime>(
    app: AppHandle<R>,
    config: ChatConfig,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    if config.api_key.trim().is_empty() {
        return Err("未配置 API Key，请先在设置中填写".to_string());
    }

    let client = Client::new();

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let base_messages = messages_to_values(&messages);

    let tools = tools_definition();

    // 第一轮：非流式请求，检测模型是否需要调用工具
    let first_body = build_body(&config, &base_messages, Some(&tools), false);

    let first_response = send_request(&client, &url, &config, &first_body).await?;

    let first_json: Value = first_response
        .json()
        .await
        .map_err(|err| format!("解析响应失败: {err}"))?;

    let message = first_json["choices"][0]["message"].clone();

    // 无工具调用：直接流式返回
    if message["tool_calls"].is_null() {
        return stream_request(&app, &client, &url, &config, &base_messages, Some(&tools)).await;
    }

    // 有工具调用：执行工具并把结果回传
    let mut all_messages = base_messages.clone();

    all_messages.push(message.clone());

    let tool_calls = message["tool_calls"].as_array().cloned().unwrap_or_default();

    for tool_call in tool_calls {
        let id = tool_call["id"].as_str().unwrap_or_default().to_string();
        let name = tool_call["function"]["name"].as_str().unwrap_or_default().to_string();
        let arguments: Value = tool_call["function"]["arguments"]
            .as_str()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or(Value::Null);

        let result = execute_tool(&app, &name, &arguments, &config)
            .unwrap_or_else(|err| format!("工具执行失败: {err}"));

        all_messages.push(json!({ "role": "tool", "tool_call_id": id, "content": result }));
    }

    // 第二轮：流式生成最终回答
    stream_request(&app, &client, &url, &config, &all_messages, None).await
}

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
