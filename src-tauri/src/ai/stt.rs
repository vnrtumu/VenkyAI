use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

type ConfigState = std::sync::Arc<parking_lot::Mutex<AppConfig>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

/// Transcribe audio using OpenAI Whisper API
pub async fn transcribe_with_openai(
    config: &AppConfig,
    audio_wav: Vec<u8>,
) -> Result<String, String> {
    if config.openai_api_key.is_empty() {
        return Err("OpenAI API key not configured".to_string());
    }

    let client = Client::new();

    let part = reqwest::multipart::Part::bytes(audio_wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("MIME error: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .text("language", "en")
        .text("response_format", "json")
        .part("file", part);

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Whisper API request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Whisper API error ({}): {}", status, body));
    }

    let result: WhisperResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Whisper response: {}", e))?;

    Ok(result.text)
}

#[tauri::command]
pub async fn transcribe_audio(
    config: tauri::State<'_, ConfigState>,
) -> Result<String, String> {
    let audio_wav = crate::capture::audio::get_audio_wav_bytes()?;
    let cfg = config.lock().clone();
    transcribe_with_openai(&cfg, audio_wav).await
}
