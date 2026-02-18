use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm_provider: LLMProvider,
    pub openai_api_key: String,
    pub openai_model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub capture_interval_secs: u64,
    pub whisper_model: String,
    pub hotkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LLMProvider {
    OpenAI,
    Ollama,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llm_provider: LLMProvider::OpenAI,
            openai_api_key: String::new(),
            openai_model: "gpt-4o".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3".to_string(),
            capture_interval_secs: 5,
            whisper_model: "base".to_string(),
            hotkey: "CmdOrCtrl+Shift+C".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load(app_data: &Path) -> Self {
        let config_path = app_data.join("config.json");
        let mut config = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            let c = Self::default();
            c.save(app_data);
            c
        };

        // Override with environment variable if set (more secure than hardcoding)
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.is_empty() {
                config.openai_api_key = key;
            }
        }

        config
    }

    pub fn save(&self, app_data: &Path) {
        let config_path = app_data.join("config.json");
        if let Ok(content) = serde_json::to_string_pretty(self) {
            std::fs::write(config_path, content).ok();
        }
    }
}

type ConfigState = std::sync::Arc<parking_lot::Mutex<AppConfig>>;

#[tauri::command]
pub fn get_config(config: tauri::State<'_, ConfigState>) -> AppConfig {
    config.lock().clone()
}

#[tauri::command]
pub fn update_config(
    app: tauri::AppHandle,
    config_state: tauri::State<'_, ConfigState>,
    new_config: AppConfig,
) -> Result<(), String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    new_config.save(&app_data);
    *config_state.lock() = new_config;
    Ok(())
}
