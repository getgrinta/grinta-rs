use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};
use base64::{Engine as _, engine::general_purpose};
use image::{DynamicImage, ImageFormat};
use icns::{IconFamily, IconType};
use tokio::process::Command;
use std::io::Cursor;

/// Extracts an application icon as a base64-encoded PNG (optimized for speed)
/// Returns None if extraction fails
/// Prioritizes smaller, faster-to-process icons
pub async fn extract_app_icon(app_path: &str) -> Option<String> {
    // Only supported on macOS
    #[cfg(target_os = "macos")]
    {
        // Check if the app path exists
        if !tokio::fs::metadata(app_path).await.is_ok() {
            return None;
        }

        // 1. Find the icon file name from Info.plist
        let icon_name = get_icon_name(app_path).await?;

        // 2. Construct the path to the icon file
        let icon_path = format!("{}/Contents/Resources/{}.icns", app_path, icon_name);
        
        if !tokio::fs::metadata(&icon_path).await.is_ok() {
            return None;
        }
        
        // 3. Read and parse the ICNS file (optimized for speed)
        match extract_small_png_from_icns(&icon_path).await {
            Ok(png_data) => {
                // 4. Encode the PNG data as base64
                let base64_icon = general_purpose::STANDARD.encode(&png_data);
                Some(base64_icon)
            },
            Err(_) => None,
        }
    }

    // Return None on non-macOS platforms
    #[cfg(not(target_os = "macos"))]
    None
}

/// Extract small PNG data from an ICNS file (optimized for speed and size)
async fn extract_small_png_from_icns(icon_path: &str) -> io::Result<Vec<u8>> {
    // Open and read the ICNS file
    let mut file = File::open(icon_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    
    // Parse the ICNS file
    let icon_family = IconFamily::read(&buffer[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse ICNS: {}", e)))?;
    
    // Prioritize smaller icons for speed (32x32, 64x64 first)
    let icon_types = [
        IconType::RGBA32_32x32,
        IconType::RGBA32_64x64,
        IconType::RGBA32_128x128,
        IconType::RGBA32_16x16,
        IconType::RGBA32_256x256, // Fallback to larger if needed
    ];
    
    // Find the first available icon type
    for &icon_type in &icon_types {
        match icon_family.get_icon_with_type(icon_type) {
            Ok(icon_element) => {
                // Get dimensions from the icon
                let width = icon_element.width();
                let height = icon_element.height();
                
                // Get icon data
                let icon_data = icon_element.data();
                
                // Create a smaller, lossy image for speed
                let image = DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(width, height, icon_data.to_vec())
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to create RGBA image"))?
                );
                
                // Resize to max 32x32 for speed and smaller payload
                let resized_image = if width > 32 || height > 32 {
                    image.resize(32, 32, image::imageops::FilterType::Triangle) // Fast triangle filter
                } else {
                    image
                };
                
                // Convert to PNG with minimal compression for speed
                let mut png_data = Vec::new();
                let mut cursor = Cursor::new(&mut png_data);
                
                resized_image.write_to(&mut cursor, ImageFormat::Png)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to encode PNG: {}", e)))?;
                
                return Ok(png_data);
            },
            Err(_) => continue,
        }
    }
    
    // If no suitable icon was found
    Err(io::Error::new(io::ErrorKind::NotFound, "No suitable icon found in ICNS file"))
}

/// Gets the icon file name from the app's Info.plist (async version)
#[cfg(target_os = "macos")]
async fn get_icon_name(app_path: &str) -> Option<String> {
    let output = Command::new("defaults")
        .args(["read", &format!("{}/Contents/Info", app_path), "CFBundleIconFile"])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let icon_name = String::from_utf8(output.stdout).ok()?;
    let icon_name = icon_name.trim();

    // Strip the `.icns` suffix if present
    Some(icon_name.strip_suffix(".icns").unwrap_or(icon_name).to_string())
}
