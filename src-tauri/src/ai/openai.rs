use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AIContext, AIResponse};
use crate::config::AppConfig;

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageResponse,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessageResponse {
    content: String,
}

pub async fn generate(
    config: &AppConfig,
    question: &str,
    _context: &AIContext,
) -> Result<AIResponse, String> {
    if config.openai_api_key.is_empty() {
        return Err("OpenAI API key not configured. Go to Settings to add your key.".to_string());
    }

    let client = Client::new();

    let request = OpenAIRequest {
        model: config.openai_model.clone(),
        messages: vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: serde_json::Value::String(
                    "You are VenkyAI, a helpful real-time AI assistant for meetings and interviews. \
                     Be concise and actionable."
                        .to_string(),
                ),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: serde_json::Value::String(question.to_string()),
            },
        ],
        max_tokens: 1024,
        temperature: 0.7,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error ({}): {}", status, body));
    }

    let body: OpenAIResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

    let content = body
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "No response from OpenAI".to_string());

    Ok(AIResponse {
        content,
        provider: "OpenAI".to_string(),
        model: config.openai_model.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn generate_with_system(
    config: &AppConfig,
    system_prompt: &str,
    question: &str,
    context: &AIContext,
) -> Result<AIResponse, String> {
    if config.openai_api_key.is_empty() {
        return Err("OpenAI API key not configured. Go to Settings to add your key.".to_string());
    }

    let client = Client::new();

    let mut messages = vec![
        OpenAIMessage {
            role: "system".to_string(),
            content: serde_json::Value::String(system_prompt.to_string()),
        },
    ];

    // If we have a screen capture, use vision
    if let Some(ref base64_img) = context.screen_base64 {
        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: serde_json::json!([
                {
                    "type": "text",
                    "text": question
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:image/png;base64,{}", base64_img),
                        "detail": "low"
                    }
                }
            ]),
        });
    } else {
        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: serde_json::Value::String(question.to_string()),
        });
    }

    let request = OpenAIRequest {
        model: config.openai_model.clone(),
        messages,
        max_tokens: 1024,
        temperature: 0.7,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error ({}): {}", status, body));
    }

    let body: OpenAIResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

    let content = body
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "No response".to_string());

    Ok(AIResponse {
        content,
        provider: "OpenAI".to_string(),
        model: config.openai_model.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
