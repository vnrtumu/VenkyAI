use reqwest::Client;
use serde::Deserialize;
use tauri::{Emitter, AppHandle};
use futures_util::StreamExt;

use crate::config::AppConfig;

type ConfigState = std::sync::Arc<parking_lot::Mutex<AppConfig>>;

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

/// Send a streaming chat request to OpenAI — emits "llm-token" events as tokens arrive
#[tauri::command]
pub async fn stream_chat(
    app: AppHandle,
    config: tauri::State<'_, ConfigState>,
    messages: Vec<crate::ai::AIMessage>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    let cfg = config.lock().clone();
    stream_llm_internal(app, cfg, messages, system_prompt).await
}

pub async fn stream_llm_internal(
    app: AppHandle,
    cfg: crate::config::AppConfig,
    messages: Vec<crate::ai::AIMessage>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    if cfg.openai_api_key.is_empty() {
        return Err("OpenAI API key not configured".to_string());
    }

    let client = Client::new();

    let mut api_messages = Vec::new();

    if let Some(sys) = system_prompt {
        api_messages.push(serde_json::json!({
            "role": "system",
            "content": sys
        }));
    }

    for msg in &messages {
        api_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    let body = serde_json::json!({
        "model": cfg.openai_model,
        "messages": api_messages,
        "stream": true
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", cfg.openai_api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Stream request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI error ({}): {}", status, body));
    }

    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut buffer = String::new();

    // Emit stream-start event
    let _ = app.emit("llm-stream-start", ());

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete SSE lines
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() || !line.starts_with("data: ") {
                continue;
            }

            let data = &line[6..];

            if data == "[DONE]" {
                let _ = app.emit("llm-stream-end", &full_response);
                return Ok(full_response);
            }

            if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                for choice in &chunk.choices {
                    if let Some(content) = &choice.delta.content {
                        full_response.push_str(content);
                        // Emit each token as it arrives
                        let _ = app.emit("llm-token", content);
                    }
                    if choice.finish_reason.is_some() {
                        let _ = app.emit("llm-stream-end", &full_response);
                        return Ok(full_response);
                    }
                }
            }
        }
    }

    let _ = app.emit("llm-stream-end", &full_response);
    Ok(full_response)
}
