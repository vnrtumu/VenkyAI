pub mod audio;
pub mod screen;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CaptureState {
    pub is_recording_audio: bool,
    pub last_screen_capture: Option<String>,
}
