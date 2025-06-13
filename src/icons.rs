use std::path::Path;
use std::fs::File;
use std::io::{self, Cursor, Read};
use base64::{Engine as _, engine::general_purpose};
use image::{DynamicImage, ImageFormat};
use icns::{IconFamily, IconType};
use std::process::Command;

/// Extracts an application icon as a base64-encoded PNG
/// Returns None if extraction fails
pub fn extract_app_icon(app_path: &str) -> Option<String> {
    // Only supported on macOS
    #[cfg(target_os = "macos")]
    {
        // Check if the app path exists
        if !Path::new(app_path).exists() {
            return None;
        }

        // 1. Find the icon file name from Info.plist
        let icon_name = get_icon_name(app_path)?;

        // 2. Construct the path to the icon file
        let icon_path = format!("{}/Contents/Resources/{}.icns", app_path, icon_name);
        
        if !Path::new(&icon_path).exists() {
            return None;
        }
        
        // 3. Read and parse the ICNS file
        match extract_png_from_icns(&icon_path) {
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

/// Extract PNG data from an ICNS file using the icns crate
fn extract_png_from_icns(icon_path: &str) -> io::Result<Vec<u8>> {
    // Open and read the ICNS file
    let mut file = File::open(icon_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    // Parse the ICNS file
    let icon_family = IconFamily::read(&buffer[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse ICNS: {}", e)))?;
    
    // Try to get the largest icon available
    // Prioritize 256x256 or 512x512 icons
    let icon_types = [
        IconType::RGBA32_512x512,
        IconType::RGBA32_256x256,
        IconType::RGBA32_128x128,
        IconType::RGBA32_64x64,
        IconType::RGBA32_32x32,
        IconType::RGBA32_16x16,
    ];
    
    // Find the first available icon type
    for &icon_type in &icon_types {
        // The get_icon_with_type method returns Result<Image, Error>
        match icon_family.get_icon_with_type(icon_type) {
            Ok(icon_element) => {
                // Get dimensions from the icon
                let width = icon_element.width();
                let height = icon_element.height();
                
                // Get icon data
                let icon_data = icon_element.data();
                
                // Create an image::DynamicImage based on the icon type
                // For RGBA32 types, we need to convert the raw data to RGBA format
                let image = match icon_type {
                    IconType::RGBA32_512x512 | IconType::RGBA32_256x256 | 
                    IconType::RGBA32_128x128 | IconType::RGBA32_64x64 | 
                    IconType::RGBA32_32x32 | IconType::RGBA32_16x16 => {
                        // The data is already in RGBA format
                        DynamicImage::ImageRgba8(
                            image::RgbaImage::from_raw(width, height, icon_data.to_vec())
                                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Failed to create RGBA image"))?
                        )
                    },
                    _ => {
                        // For other formats, we'd need different conversion logic
                        // But we're only trying RGBA32 formats, so this shouldn't happen
                        return Err(io::Error::other("Unsupported icon format"));
                    }
                };
                
                // Convert to PNG
                let mut png_data = Vec::new();
                let mut cursor = Cursor::new(&mut png_data);
                
                // Handle potential image error
                image.write_to(&mut cursor, ImageFormat::Png)
                    .map_err(|e| io::Error::other(format!("Failed to encode PNG: {}", e)))?;
                
                return Ok(png_data);
            },
            Err(_) => continue,
        }
    }
    
    // If no suitable icon was found
    Err(io::Error::other("No suitable icon found in ICNS file"))
}

/// Gets the icon file name from the app's Info.plist
#[cfg(target_os = "macos")]
fn get_icon_name(app_path: &str) -> Option<String> {
    let output = Command::new("defaults")
        .args(["read", &format!("{}/Contents/Info", app_path), "CFBundleIconFile"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let icon_name = String::from_utf8(output.stdout).ok()?;
    let icon_name = icon_name.trim();

    // Strip the `.icns` suffix if present using the standard helper
    Some(icon_name.strip_suffix(".icns").unwrap_or(icon_name).to_string())
}
