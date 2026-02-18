use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::AIResponse;
use crate::config::AppConfig;

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: Option<OllamaMessageResponse>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessageResponse {
    content: String,
}

pub async fn generate(
    config: &AppConfig,
    question: &str,
    _context: &super::AIContext,
) -> Result<AIResponse, String> {
    let client = Client::new();

    let request = OllamaRequest {
        model: config.ollama_model.clone(),
        messages: vec![
            OllamaMessage {
                role: "system".to_string(),
                content: "You are VenkyAI, a helpful real-time AI assistant for meetings \
                          and interviews. Be concise and actionable."
                    .to_string(),
            },
            OllamaMessage {
                role: "user".to_string(),
                content: question.to_string(),
            },
        ],
        stream: false,
    };

    let url = format!("{}/api/chat", config.ollama_url);

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}. Is Ollama running?", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama API error ({}): {}", status, body));
    }

    let body: OllamaResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let content = body
        .message
        .map(|m| m.content)
        .unwrap_or_else(|| "No response from Ollama".to_string());

    Ok(AIResponse {
        content,
        provider: "Ollama".to_string(),
        model: config.ollama_model.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn generate_with_system(
    config: &AppConfig,
    system_prompt: &str,
    question: &str,
) -> Result<AIResponse, String> {
    let client = Client::new();

    let request = OllamaRequest {
        model: config.ollama_model.clone(),
        messages: vec![
            OllamaMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            OllamaMessage {
                role: "user".to_string(),
                content: question.to_string(),
            },
        ],
        stream: false,
    };

    let url = format!("{}/api/chat", config.ollama_url);

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}. Is Ollama running?", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama API error ({}): {}", status, body));
    }

    let body: OllamaResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let content = body
        .message
        .map(|m| m.content)
        .unwrap_or_else(|| "No response".to_string());

    Ok(AIResponse {
        content,
        provider: "Ollama".to_string(),
        model: config.ollama_model.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
