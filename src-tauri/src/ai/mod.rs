pub mod llm;
pub mod ollama;
pub mod openai;
pub mod stt;
pub mod streaming;
pub mod live_engine;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIContext {
    pub transcript: Option<String>,
    pub screen_description: Option<String>,
    pub custom_prompt: Option<String>,
    pub screen_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub timestamp: String,
}
