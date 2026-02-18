use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::io::Cursor;

#[derive(Debug, Serialize)]
pub struct ScreenCapture {
    pub base64_image: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: String,
}

#[tauri::command]
pub fn capture_screen() -> Result<ScreenCapture, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("Failed to list monitors: {}", e))?;

    let monitor = monitors
        .first()
        .ok_or_else(|| "No monitors found".to_string())?;

    let image = monitor
        .capture_image()
        .map_err(|e| format!("Failed to capture screen: {}", e))?;

    let width = image.width();
    let height = image.height();

    // Resize for efficiency (max 1280px wide)
    let image = if width > 1280 {
        let scale = 1280.0 / width as f64;
        let new_height = (height as f64 * scale) as u32;
        image::imageops::resize(
            &image,
            1280,
            new_height,
            image::imageops::FilterType::Triangle,
        )
    } else {
        image
    };

    // Encode to PNG then base64
    let mut buffer = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode image: {}", e))?;

    let base64_image = STANDARD.encode(buffer.into_inner());

    Ok(ScreenCapture {
        base64_image,
        width,
        height,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
