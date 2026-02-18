use serde::{Deserialize, Serialize};

use super::{AIContext, AIResponse};
use crate::config::{AppConfig, LLMProvider};

type ConfigState = std::sync::Arc<parking_lot::Mutex<AppConfig>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub available: bool,
    pub models: Vec<String>,
}

fn build_system_prompt(context: &AIContext) -> String {
    let mut prompt = String::from(
        "You are VenkyAI, a real-time AI assistant helping the user during virtual meetings, \
         interviews, and presentations. Provide concise, actionable suggestions. \
         Be direct and helpful.\n\n",
    );

    if let Some(ref custom) = context.custom_prompt {
        prompt.push_str("## Custom Instructions\n");
        prompt.push_str(custom);
        prompt.push_str("\n\n");
    }

    if let Some(ref transcript) = context.transcript {
        prompt.push_str("## Current Conversation Transcript\n");
        prompt.push_str(transcript);
        prompt.push_str("\n\n");
    }

    if let Some(ref desc) = context.screen_description {
        prompt.push_str("## What's Currently on Screen\n");
        prompt.push_str(desc);
        prompt.push_str("\n\n");
    }

    prompt.push_str(
        "Based on all the context above, provide helpful suggestions, talking points, \
         or answers the user might need right now. Be concise and practical.",
    );

    prompt
}

#[tauri::command]
pub async fn ask_ai(
    config: tauri::State<'_, ConfigState>,
    question: String,
) -> Result<AIResponse, String> {
    let cfg = config.lock().clone();

    let context = AIContext {
        transcript: None,
        screen_description: None,
        custom_prompt: None,
        screen_base64: None,
    };

    match cfg.llm_provider {
        LLMProvider::OpenAI => {
            super::openai::generate(&cfg, &question, &context).await
        }
        LLMProvider::Ollama => {
            super::ollama::generate(&cfg, &question, &context).await
        }
    }
}

#[tauri::command]
pub async fn ask_ai_with_context(
    config: tauri::State<'_, ConfigState>,
    question: String,
    context: AIContext,
) -> Result<AIResponse, String> {
    let cfg = config.lock().clone();

    let system_prompt = build_system_prompt(&context);

    match cfg.llm_provider {
        LLMProvider::OpenAI => {
            super::openai::generate_with_system(&cfg, &system_prompt, &question, &context).await
        }
        LLMProvider::Ollama => {
            super::ollama::generate_with_system(&cfg, &system_prompt, &question).await
        }
    }
}

#[tauri::command]
pub fn get_available_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            name: "OpenAI".to_string(),
            available: true,
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
        },
        ProviderInfo {
            name: "Ollama".to_string(),
            available: true,
            models: vec![
                "llama3".to_string(),
                "mistral".to_string(),
                "codellama".to_string(),
                "gemma".to_string(),
            ],
        },
    ]
}
