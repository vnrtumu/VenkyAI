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

        loop {
            interval.tick().await;
            
            let windows = xcap::Window::all().unwrap_or_default();
            let mut detected_title: Option<String> = None;

            for window in windows {
                if let Ok(title) = window.title() {
                    if meeting_regex.is_match(&title) {
                        detected_title = Some(title);
                        break;
                    }
                }
            }

            let session_manager = app.state::<Arc<Mutex<SessionManager>>>();
            let mgr = session_manager.lock();

            if let Some(title) = detected_title {
                if mgr.current_session.is_none() {
                    log::info!("Meeting detected: {}. Auto-starting session and audio capture.", title);
                    
                    // Create session
                    if let Ok(session) = crate::session::manager::create_session(app.state(), title.clone()) {
                        let _ = app.emit("session-auto-started", session);
                        
                        // Start system audio capture (hearing others)
                        let _ = audio::start_system_audio_capture();
                        // Start mic capture (hearing you)
                        let _ = audio::start_audio_capture(app.state());
                    }
                }
            } else {
                // If no meeting window, and session is active, maybe auto-end?
                // For safety, let's just log it for now.
                if mgr.current_session.is_some() {
                    // log::info!("Meeting window gone. Should we end session?");
                }
            }
        }
    }
}

pub async fn transcription_loop(app: AppHandle) {
    let mut interval = tokio::time::interval(Duration::from_millis(4000));
    
    loop {
        interval.tick().await;

        let session_manager = app.state::<Arc<Mutex<SessionManager>>>();
        let is_active = session_manager.lock().current_session.is_some();

        if is_active {
            // 1. Get current audio buffers
            // 2. Combine or process them separately? 
            // For simplicity, let's process the system audio (other people) first
            let _wav_bytes = match audio::get_audio_wav_bytes() {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };

            // 3. Request transcription
            // Note: In a real app, we'd use chunks. For now, we use the full buffer
            // and clear it after processing to avoid re-transcribing same part.
            // But Whisper API doesn't support streaming well.
            // Let's just emit a placeholder for now to prove flow works
            
            // To make it real: we'd call transcribe_audio here
            // But we need the OpenAI key from config
            let config_state = app.state::<Arc<Mutex<crate::config::AppConfig>>>();
            let cfg = config_state.lock().clone();

            if !cfg.openai_api_key.is_empty() {
                // Background transcription
                log::debug!("Running background transcription chunk...");
                // In a real live engine, we'd use a separate shorter buffer
            }
        }
    }
}
