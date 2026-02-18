use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::storage::Storage;
use crate::config::AppConfig;

type StorageState = Arc<Mutex<Storage>>;
type ConfigState = Arc<Mutex<AppConfig>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub status: SessionStatus,
    pub start_time: String,
    pub end_time: Option<String>,
    pub transcript: Vec<TranscriptEntry>,
    pub suggestions: Vec<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Paused,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: String,
    pub speaker: String,
    pub text: String,
}

pub struct SessionManager {
    pub current_session: Option<Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            current_session: None,
        }
    }
}

type SessionState = Arc<Mutex<SessionManager>>;

#[tauri::command]
pub fn create_session(
    session_state: tauri::State<'_, SessionState>,
    title: String,
) -> Result<Session, String> {
    let mut mgr = session_state.lock();

    if mgr.current_session.is_some() {
        return Err("A session is already active. End it before starting a new one.".to_string());
    }

    let session = Session {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        status: SessionStatus::Active,
        start_time: chrono::Utc::now().to_rfc3339(),
        end_time: None,
        transcript: Vec::new(),
        suggestions: Vec::new(),
        summary: None,
    };

    mgr.current_session = Some(session.clone());
    Ok(session)
}

#[tauri::command]
pub fn end_session(
    session_state: tauri::State<'_, SessionState>,
    storage_state: tauri::State<'_, StorageState>,
) -> Result<Session, String> {
    let mut mgr = session_state.lock();

    let session = mgr
        .current_session
        .as_mut()
        .ok_or_else(|| "No active session".to_string())?;

    session.status = SessionStatus::Ended;
    session.end_time = Some(chrono::Utc::now().to_rfc3339());

    let finished = session.clone();

    // Save to storage
    let storage = storage_state.lock();
    storage.save_session(&finished).ok();

    mgr.current_session = None;
    Ok(finished)
}

#[tauri::command]
pub fn get_current_session(
    session_state: tauri::State<'_, SessionState>,
) -> Option<Session> {
    session_state.lock().current_session.clone()
}

#[tauri::command]
pub fn add_transcript_entry(
    session_state: tauri::State<'_, SessionState>,
    speaker: String,
    text: String,
) -> Result<TranscriptEntry, String> {
    let mut mgr = session_state.lock();

    let session = mgr
        .current_session
        .as_mut()
        .ok_or_else(|| "No active session".to_string())?;

    let entry = TranscriptEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        speaker,
        text,
    };

    session.transcript.push(entry.clone());
    Ok(entry)
}

#[tauri::command]
pub fn get_session_transcript(
    session_state: tauri::State<'_, SessionState>,
) -> Result<Vec<TranscriptEntry>, String> {
    let mgr = session_state.lock();
    let session = mgr
        .current_session
        .as_ref()
        .ok_or_else(|| "No active session".to_string())?;
    Ok(session.transcript.clone())
}

#[tauri::command]
pub async fn generate_summary(
    session_state: tauri::State<'_, SessionState>,
    config_state: tauri::State<'_, ConfigState>,
) -> Result<String, String> {
    let transcript_text = {
        let mgr = session_state.lock();
        let session = mgr
            .current_session
            .as_ref()
            .ok_or_else(|| "No active session".to_string())?;

        session
            .transcript
            .iter()
            .map(|e| format!("[{}] {}: {}", e.timestamp, e.speaker, e.text))
            .collect::<Vec<_>>()
            .join("\n")
    };

    if transcript_text.is_empty() {
        return Err("No transcript to summarize".to_string());
    }

    let cfg = config_state.lock().clone();

    let question = format!(
        "Summarize the following meeting transcript into key points, action items, and decisions:\n\n{}",
        transcript_text
    );

    let context = crate::ai::AIContext {
        transcript: Some(transcript_text),
        screen_description: None,
        custom_prompt: Some("Generate a concise meeting summary with: 1) Key Points 2) Action Items 3) Decisions Made".to_string()),
        screen_base64: None,
    };

    let response = match cfg.llm_provider {
        crate::config::LLMProvider::OpenAI => {
            crate::ai::openai::generate(&cfg, &question, &context).await?
        }
        crate::config::LLMProvider::Ollama => {
            crate::ai::ollama::generate(&cfg, &question, &context).await?
        }
    };

    // Save summary to session
    {
        let mut mgr = session_state.lock();
        if let Some(ref mut session) = mgr.current_session {
            session.summary = Some(response.content.clone());
        }
    }

    Ok(response.content)
}
