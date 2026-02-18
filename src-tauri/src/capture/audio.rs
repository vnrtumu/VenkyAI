use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;

use super::CaptureState;

type CaptureStateHandle = Arc<Mutex<CaptureState>>;

/// Wrapper to make cpal::Stream Send+Sync (it is safe for our usage pattern)
struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

/// Shared audio buffer that collects samples during recording
static AUDIO_BUFFER: once_cell::sync::Lazy<Arc<Mutex<Vec<f32>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Active audio stream handle (kept alive while recording)
static AUDIO_STREAM: once_cell::sync::Lazy<Arc<Mutex<Option<SendStream>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

static SAMPLE_RATE: once_cell::sync::Lazy<Arc<Mutex<u32>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(44100)));

/// System audio buffer (from scab)
static SYSTEM_AUDIO_BUFFER: once_cell::sync::Lazy<Arc<Mutex<Vec<f32>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Handle for scap recorder thread
static SYSTEM_AUDIO_THREAD: once_cell::sync::Lazy<Arc<Mutex<Option<std::thread::JoinHandle<()>>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

static STOP_SIGNAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Serialize)]
pub struct AudioStatus {
    pub is_recording: bool,
    pub is_recording_system: bool,
    pub buffer_duration_secs: f32,
    pub system_buffer_duration_secs: f32,
    pub sample_rate: u32,
}

#[tauri::command]
pub fn start_audio_capture(
    state: tauri::State<'_, CaptureStateHandle>,
) -> Result<String, String> {
    let mut capture_state = state.lock();
    if capture_state.is_recording_audio {
        return Err("Already recording".to_string());
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No input device available".to_string())?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get input config: {}", e))?;

    let sr = config.sample_rate().0;
    *SAMPLE_RATE.lock() = sr;

    // Clear previous buffer
    AUDIO_BUFFER.lock().clear();

    let buffer = AUDIO_BUFFER.clone();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        buffer.lock().extend_from_slice(data);
                    },
                    |err| {
                        log::error!("Audio stream error: {}", err);
                    },
                    None,
                )
                .map_err(|e| format!("Failed to build stream: {}", e))?;
            stream
        }
        cpal::SampleFormat::I16 => {
            let stream = device
                .build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let floats: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        buffer.lock().extend_from_slice(&floats);
                    },
                    |err| {
                        log::error!("Audio stream error: {}", err);
                    },
                    None,
                )
                .map_err(|e| format!("Failed to build stream: {}", e))?;
            stream
        }
        format => {
            return Err(format!("Unsupported sample format: {:?}", format));
        }
    };

    stream
        .play()
        .map_err(|e| format!("Failed to start stream: {}", e))?;

    *AUDIO_STREAM.lock() = Some(SendStream(stream));
    capture_state.is_recording_audio = true;

    Ok(format!(
        "Recording started (device: {}, sample rate: {}Hz)",
        device.name().unwrap_or_default(),
        sr
    ))
}

#[tauri::command]
pub fn stop_audio_capture(
    state: tauri::State<'_, CaptureStateHandle>,
) -> Result<Vec<f32>, String> {
    let mut capture_state = state.lock();
    if !capture_state.is_recording_audio {
        return Err("Not recording".to_string());
    }

    // Drop the stream to stop recording
    *AUDIO_STREAM.lock() = None;
    capture_state.is_recording_audio = false;

    // Return the captured audio buffer
    let buffer = AUDIO_BUFFER.lock().clone();
    Ok(buffer)
}

#[tauri::command]
pub fn start_system_audio_capture() -> Result<String, String> {
    // Check if system audio capture thread is already running
    if SYSTEM_AUDIO_THREAD.lock().is_some() {
        return Err("System audio recording already active".to_string());
    }

    // Initialize scap
    if !scap::has_permission() {
        return Err("System audio capture permission not granted".to_string());
    }

    let options = scap::capturer::Options {
        fps: 1, 
        show_cursor: false,
        captures_audio: true,
        ..Default::default()
    };

    let buffer = SYSTEM_AUDIO_BUFFER.clone();
    buffer.lock().clear();

    let mut capturer = scap::capturer::Capturer::build(options)
        .map_err(|e| format!("Failed to build scap capturer: {:?}", e))?;

    capturer.start_capture();
    STOP_SIGNAL.store(false, std::sync::atomic::Ordering::SeqCst);

    // Spawn a background thread to poll for audio frames
    let handle = std::thread::spawn(move || {
        while !STOP_SIGNAL.load(std::sync::atomic::Ordering::SeqCst) {
            match capturer.get_next_frame() {
                Ok(frame) => {
                    if let scap::frame::Frame::Audio(audio_frame) = frame {
                        let data = audio_frame.raw_data();
                        if matches!(audio_frame.format(), scap::frame::AudioFormat::F32) {
                            let floats: Vec<f32> = data
                                .chunks_exact(4)
                                .map(|c| f32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                                .collect();
                            buffer.lock().extend_from_slice(&floats);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error getting next frame: {:?}", e);
                    break;
                }
            }
        }
        log::info!("System audio polling loop ended");
    });

    *SYSTEM_AUDIO_THREAD.lock() = Some(handle);

    Ok("System audio capture started".to_string())
}

#[tauri::command]
pub fn stop_system_audio_capture() -> Result<Vec<f32>, String> {
    STOP_SIGNAL.store(true, std::sync::atomic::Ordering::SeqCst);
    
    let mut thread_handle = SYSTEM_AUDIO_THREAD.lock();
    if let Some(handle) = thread_handle.take() {
        let _ = handle.join();
        let buffer = SYSTEM_AUDIO_BUFFER.lock().clone();
        Ok(buffer)
    } else {
        Err("System audio capture not active".to_string())
    }
}

#[tauri::command]
pub fn get_audio_status(state: tauri::State<'_, CaptureStateHandle>) -> AudioStatus {
    let capture_state = state.lock();
    let buffer_len = AUDIO_BUFFER.lock().len();
    let system_buffer_len = SYSTEM_AUDIO_BUFFER.lock().len();
    let sr = *SAMPLE_RATE.lock();
    AudioStatus {
        is_recording: capture_state.is_recording_audio,
        is_recording_system: SYSTEM_AUDIO_THREAD.lock().is_some(),
        buffer_duration_secs: buffer_len as f32 / sr as f32,
        system_buffer_duration_secs: system_buffer_len as f32 / sr as f32,
        sample_rate: sr,
    }
}

/// Get the current audio buffer as WAV bytes and CLEAR the buffer
pub fn get_and_clear_audio_wav_bytes() -> Result<Vec<u8>, String> {
    let mut buffer_lock = AUDIO_BUFFER.lock();
    if buffer_lock.is_empty() {
        return Err("No audio data".to_string());
    }
    
    let buffer = std::mem::take(&mut *buffer_lock);
    drop(buffer_lock); // Release lock early

    let sr = *SAMPLE_RATE.lock();

    let mut cursor = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer =
        hound::WavWriter::new(&mut cursor, spec).map_err(|e| format!("WAV error: {}", e))?;

    for &sample in &buffer {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(s)
            .map_err(|e| format!("WAV write error: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("WAV finalize error: {}", e))?;

    Ok(cursor.into_inner())
}

/// Get the current audio buffer as WAV bytes (for STT processing)
#[allow(dead_code)]
pub fn get_audio_wav_bytes() -> Result<Vec<u8>, String> {
    let buffer = AUDIO_BUFFER.lock().clone();
    let sr = *SAMPLE_RATE.lock();

    if buffer.is_empty() {
        return Err("No audio data".to_string());
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer =
        hound::WavWriter::new(&mut cursor, spec).map_err(|e| format!("WAV error: {}", e))?;

    for &sample in &buffer {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(s)
            .map_err(|e| format!("WAV write error: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("WAV finalize error: {}", e))?;

    Ok(cursor.into_inner())
}
