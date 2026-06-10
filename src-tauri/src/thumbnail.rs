use image::imageops::FilterType;
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Generate a deterministic hash of a file path for use as thumbnail filename.
pub fn hash_path(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a thumbnail for the image at `src_path` and save it to `dst_path`.
///
/// The thumbnail will be at most `max_size` pixels on its longest side,
/// preserving aspect ratio. Output is always JPEG for consistency.
pub fn generate_thumbnail(src_path: &Path, dst_path: &Path, max_size: u32) -> Result<(), String> {
    let img = image::open(src_path).map_err(|e| format!("Cannot open image: {e}"))?;

    let (w, h) = img.dimensions();

    // Calculate new dimensions preserving aspect ratio
    let (new_w, new_h) = if w > h {
        let ratio = max_size as f32 / w as f32;
        (max_size, (h as f32 * ratio).round().max(1.0) as u32)
    } else {
        let ratio = max_size as f32 / h as f32;
        ((w as f32 * ratio).round().max(1.0) as u32, max_size)
    };

    // Only downscale, never upscale
    let resized = if w > max_size || h > max_size {
        img.resize(new_w, new_h, FilterType::Lanczos3)
    } else {
        img
    };

    // Convert to RGB8 for JPEG output (handles RGBA, grayscale, etc.)
    let rgb = resized.to_rgb8();

    rgb.save_with_format(dst_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Cannot save thumbnail: {e}"))?;

    Ok(())
}
