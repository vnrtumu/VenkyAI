use std::sync::Arc;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager, Emitter};
use regex::Regex;
use std::time::Duration;
use tokio::time;

use crate::session::manager::SessionManager;
use crate::capture::audio;

pub struct LiveEngine;

impl LiveEngine {
    pub async fn start_monitoring(app: AppHandle) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        // Common meeting window titles
        let meeting_regex = Regex::new(r"(?i)(Meet -|Zoom Meeting|Microsoft Teams|Webex|GoToMeeting)").unwrap();
        let mut last_detected_title: Option<String> = None;

        loop {
            interval.tick().await;
            
            let windows = xcap::Window::all().unwrap_or_default();
            let mut current_detected_title: Option<String> = None;

            for window in windows {
                if let Ok(title) = window.title() {
                    if meeting_regex.is_match(&title) {
                        current_detected_title = Some(title);
                        break;
                    }
                }
            }

            // Emit detection event if it's a new meeting
            if let Some(ref title) = current_detected_title {
                if last_detected_title.as_ref() != Some(title) {
                    log::info!("Meeting detected: {}", title);
                    let _ = app.emit("meeting-detected", title.clone());
                    last_detected_title = Some(title.clone());
                }
            } else {
                last_detected_title = None;
            }

            let session_manager = app.state::<Arc<Mutex<SessionManager>>>();
            let should_start = {
                let mgr = session_manager.lock();
                mgr.current_session.is_none()
            };

            if let Some(title) = current_detected_title {
                if should_start {
                    log::info!("Meeting detected: {}. Auto-starting session and audio capture.", title);
                    
                    // Create session - This also acquires the lock, so we must not hold it here!
                    if let Ok(session) = crate::session::manager::create_session(app.state(), title.clone(), "meeting".to_string(), None) {
                        let _ = app.emit("session-auto-started", session);
                        
                        // Start system audio capture (hearing others)
                        let _ = audio::start_system_audio_capture();
                        // Start mic capture (hearing you)
                        let _ = audio::start_audio_capture(app.state());
                    }
                }
            }
        }
    }
}

pub async fn transcription_loop(app: AppHandle) {
    let mut interval = time::interval(Duration::from_millis(1500)); // Reduced from 4s to 1.5s
    
    loop {
        interval.tick().await;

        let session_manager = app.state::<Arc<Mutex<SessionManager>>>();
        let is_active = session_manager.lock().current_session.is_some();

        if is_active {
            // 1. Get current audio chunks and clear the buffer
            let wav_bytes = match audio::get_and_clear_audio_wav_bytes() {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };

            // 2. Transcribe
            let config_state = app.state::<Arc<Mutex<crate::config::AppConfig>>>();
            let cfg = config_state.lock().clone();

            if !cfg.openai_api_key.is_empty() {
                let app_handle = app.clone();
                tokio::spawn(async move {
                    log::debug!("Running background transcription chunk...");
                    match crate::ai::stt::transcribe_with_openai(&cfg, wav_bytes).await {
                        Ok(text) => {
                            if !text.trim().is_empty() {
                                log::debug!("Transcription chunk: {}", text);
                                let _ = app_handle.emit("transcription-chunk", text);
                            }
                        }
                        Err(e) => {
                            log::error!("Background transcription error: {}", e);
                        }
                    }
                });
            }
        }
    }
}

pub async fn suggestion_loop(app: AppHandle) {
    let mut interval = time::interval(Duration::from_secs(2)); // Reduced from 8s to 2s
    let mut last_processed_count = 0;

    loop {
        interval.tick().await;

        let session_manager = app.state::<Arc<Mutex<SessionManager>>>();
        let (transcript_text, current_count, purpose, context) = {
            let mgr = session_manager.lock();
            if let Some(ref session) = mgr.current_session {
                let text = session.transcript
                    .iter()
                    .rev()
                    .take(15) // Take last 15 entries for more context
                    .rev()
                    .map(|e| format!("{}: {}", e.speaker, e.text))
                    .collect::<Vec<_>>()
                    .join("\n");
                (text, session.transcript.len(), session.purpose.clone(), session.context.clone())
            } else {
                (String::new(), 0, String::new(), None)
            }
        };

        if !transcript_text.is_empty() && current_count > last_processed_count {
            last_processed_count = current_count;

            let config_state = app.state::<Arc<Mutex<crate::config::AppConfig>>>();
            let cfg = config_state.lock().clone();

            if !cfg.openai_api_key.is_empty() {
                let app_handle = app.clone();
                tokio::spawn(async move {
                    log::debug!("Generating automated answer...");
                    
                    let mut system_prompt = format!(
                        "You are VenkyAI, a world-class AI assistant helping the user during a {}. \
                         Your primary goal is to provide perfectly tailored answers to interviewer questions. \
                         Detect if a question was JUST asked and provide the best response for the user to say. \
                         Be direct. NO prefixes like 'Answer:'. \
                         Respond with '[SILENCE]' if no response is needed right now. \
                         Be concise (max 3 sentences).",
                        purpose.to_uppercase()
                    );

                    if let Some(ref ctx) = context {
                        system_prompt.push_str(&format!("\n\n## User's Resume/Context:\n{}", ctx));
                    }

                    system_prompt.push_str(&format!("\n\n## Recent Transcript:\n{}", transcript_text));

                    let messages = vec![crate::ai::AIMessage {
                        role: "user".to_string(),
                        content: "What is the best answer or talking point for the current moment?".to_string(),
                    }];

                    match crate::ai::streaming::stream_llm_internal(app_handle.clone(), cfg, messages, Some(system_prompt)).await {
                        Ok(full_response) => {
                            if !full_response.contains("[SILENCE]") && !full_response.trim().is_empty() {
                                log::debug!("Automated streaming response complete.");
                            }
                        }
                        Err(e) => {
                            log::error!("Automated suggestion error: {}", e);
                        }
                    }
                });
            }
        }
    }
}
