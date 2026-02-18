mod ai;
mod capture;
mod config;
mod integrations;
mod session;

use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // ─── State Management ────────────────────────────────────────
            let app_data = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_data).ok();

            // App config
            let app_config = config::AppConfig::load(&app_data);
            let config_state = Arc::new(Mutex::new(app_config));
            app.manage(config_state);

            // Capture state
            let capture_state = Arc::new(Mutex::new(capture::CaptureState::default()));
            app.manage(capture_state);

            // Session state
            let session_state = Arc::new(Mutex::new(
                session::manager::SessionManager::new(),
            ));
            app.manage(session_state);

            // CRM state
            let crm_state = Arc::new(Mutex::new(integrations::CRMConfig::default()));
            app.manage(crm_state);

            // ─── System Tray ─────────────────────────────────────────────
            let show_item = MenuItem::with_id(app, "show", "Show VenkyAI", true, None::<&str>)
                .expect("failed to create menu item");
            let hide_item = MenuItem::with_id(app, "hide", "Hide VenkyAI", true, None::<&str>)
                .expect("failed to create menu item");
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)
                .expect("failed to create menu item");

            let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])
                .expect("failed to create menu");

            // Load icon from embedded bytes — decode the PNG to raw RGBA
            let icon_png_bytes = include_bytes!("../icons/32x32.png");
            let icon_image = image::load_from_memory(icon_png_bytes)
                .expect("Failed to decode icon PNG");
            let rgba = icon_image.to_rgba8();
            let (w, h) = rgba.dimensions();
            let tray_icon = tauri::image::Image::new_owned(rgba.into_raw(), w, h);

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .tooltip("VenkyAI — AI Meeting Assistant")
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("overlay") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("overlay") {
                            let _ = window.hide();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)
                .expect("Failed to create tray icon");

            // ─── Global Hotkey (Cmd+Shift+C / Ctrl+Shift+C) ─────────────
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

            let app_handle = app.handle().clone();
            app.global_shortcut().on_shortcut("CmdOrCtrl+Shift+C", move |_app, _shortcut, _event| {
                if let Some(window) = app_handle.get_webview_window("overlay") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                        let _ = app_handle.emit("overlay-visibility", false);
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = app_handle.emit("overlay-visibility", true);
                    }
                }
            }).expect("Failed to register global shortcut");

            log::info!("VenkyAI initialized. Hotkey: Cmd/Ctrl+Shift+C");

            // Spawn background monitoring tasks
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                ai::live_engine::LiveEngine::start_monitoring(handle).await;
            });

            let handle_transcribe = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                ai::live_engine::transcription_loop(handle_transcribe).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Config
            config::get_config,
            config::update_config,
            // Screen capture
            capture::screen::capture_screen,
            // Audio capture
            capture::audio::start_audio_capture,
            capture::audio::stop_audio_capture,
            capture::audio::get_audio_status,
            capture::audio::start_system_audio_capture,
            capture::audio::stop_system_audio_capture,
            // AI / LLM
            ai::llm::ask_ai,
            ai::llm::ask_ai_with_context,
            ai::llm::get_available_providers,
            // Speech-to-text
            ai::stt::transcribe_audio,
            // Streaming
            ai::streaming::stream_chat,
            // Session management
            session::manager::create_session,
            session::manager::end_session,
            session::manager::add_transcript_entry,
            session::manager::get_current_session,
            session::manager::get_session_transcript,
            session::manager::generate_summary,
            // CRM integration
            integrations::crm::get_crm_config,
            integrations::crm::update_crm_config,
            integrations::crm::crm_sync_contact,
            integrations::crm::crm_sync_notes,
            integrations::crm::get_crm_providers,
            // Overlay control
            toggle_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VenkyAI");
}

#[tauri::command]
fn toggle_overlay(app: tauri::AppHandle) -> Result<bool, String> {
    let window = app
        .get_webview_window("overlay")
        .ok_or("Overlay window not found")?;

    let visible = window.is_visible().unwrap_or(false);
    if visible {
        window.hide().map_err(|e| e.to_string())?;
    } else {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }

    let _ = app.emit("overlay-visibility", !visible);
    Ok(!visible)
}
