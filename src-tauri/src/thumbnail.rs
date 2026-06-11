use sha2::{Digest, Sha256};
use std::path::Path;

/// Maximum allowed dimension for a JPEG header value (safety cap).
const MAX_DIMENSION: usize = 65535;

/// Generate a deterministic hash of a file path for use as thumbnail filename.
pub fn hash_path(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a thumbnail for the image at `src_path` and save it to `dst_path`.
///
/// JPEG files use turbojpeg's DCT-domain scaling for ~25x speedup.
/// PNG/WebP fall back to the image crate.
pub fn generate_thumbnail(
    src_path: &Path,
    dst_path: &Path,
    max_size: u32,
) -> Result<(), String> {
    let ext = src_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "jpg" || ext == "jpeg" {
        generate_thumbnail_turbojpeg(src_path, dst_path, max_size)
    } else {
        generate_thumbnail_image(src_path, dst_path, max_size)
    }
}

/// Validate and cap a dimension value from JPEG header.
fn safe_dimension(val: usize, name: &str) -> Result<u32, String> {
    if val == 0 || val > MAX_DIMENSION {
        return Err(format!("Invalid JPEG {}: {}", name, val));
    }
    Ok(val as u32)
}

/// Fast path: turbojpeg with DCT-domain scaling.
fn generate_thumbnail_turbojpeg(
    src_path: &Path,
    dst_path: &Path,
    max_size: u32,
) -> Result<(), String> {
    let jpeg_data = std::fs::read(src_path)
        .map_err(|e| format!("Cannot read file: {e}"))?;

    // Read and validate original dimensions
    let header = turbojpeg::read_header(&jpeg_data)
        .map_err(|e| format!("Cannot read JPEG header: {e}"))?;
    let orig_w = safe_dimension(header.width, "width")?;
    let orig_h = safe_dimension(header.height, "height")?;

    // Choose the best DCT scaling factor
    let longest = orig_w.max(orig_h);
    let scale = if longest <= max_size {
        turbojpeg::ScalingFactor::ONE
    } else if longest / 2 <= max_size {
        turbojpeg::ScalingFactor::ONE_HALF
    } else if longest / 4 <= max_size {
        turbojpeg::ScalingFactor::ONE_QUARTER
    } else {
        turbojpeg::ScalingFactor::ONE_EIGHTH
    };

    // Create decompressor with scaling factor
    let mut decompressor = turbojpeg::Decompressor::new()
        .map_err(|e| format!("Cannot create decompressor: {e}"))?;
    let _ = decompressor.set_scaling_factor(scale);

    // Get scaled dimensions
    let scaled_header = decompressor
        .read_header(&jpeg_data)
        .map_err(|e| format!("Cannot read header: {e}"))?
        .scaled(scale);

    let scaled_w = safe_dimension(scaled_header.width, "scaled_width")?;
    let scaled_h = safe_dimension(scaled_header.height, "scaled_height")?;

    // Decompress with DCT scaling — checked arithmetic prevents overflow
    let pitch = (scaled_w as usize)
        .checked_mul(3)
        .ok_or("Thumbnail pitch overflow")?;
    let buf_size = (scaled_h as usize)
        .checked_mul(pitch)
        .ok_or("Thumbnail buffer size overflow")?;
    let mut pixels = vec![0u8; buf_size];
    let output = turbojpeg::Image {
        pixels: pixels.as_mut_slice(),
        width: scaled_w as usize,
        pitch,
        height: scaled_h as usize,
        format: turbojpeg::PixelFormat::RGB,
    };
    decompressor
        .decompress(&jpeg_data, output)
        .map_err(|e| format!("Cannot decompress JPEG: {e}"))?;

    // Final resize if DCT scaling wasn't precise enough
    let (final_w, final_h) = compute_target_size(scaled_w, scaled_h, max_size);
    let final_pixels = if final_w != scaled_w || final_h != scaled_h {
        resize_bilinear(&pixels, scaled_w, scaled_h, final_w, final_h)
    } else {
        pixels
    };

    // Encode as JPEG
    let mut compressor = turbojpeg::Compressor::new()
        .map_err(|e| format!("Cannot create compressor: {e}"))?;
    compressor.set_quality(80)
        .map_err(|e| format!("Cannot set quality: {e}"))?;
    compressor.set_subsamp(turbojpeg::Subsamp::Sub2x2)
        .map_err(|e| format!("Cannot set subsamp: {e}"))?;

    let img = turbojpeg::Image {
        pixels: final_pixels.as_slice(),
        width: final_w as usize,
        pitch: final_w as usize * 3,
        height: final_h as usize,
        format: turbojpeg::PixelFormat::RGB,
    };

    let compressed = compressor
        .compress_to_vec(img)
        .map_err(|e| format!("Cannot encode JPEG: {e}"))?;

    std::fs::write(dst_path, &compressed)
        .map_err(|e| format!("Cannot write thumbnail: {e}"))?;

    Ok(())
}

/// Fallback for PNG/WebP: use the image crate.
fn generate_thumbnail_image(
    src_path: &Path,
    dst_path: &Path,
    max_size: u32,
) -> Result<(), String> {
    use image::GenericImageView;

    let img = image::open(src_path).map_err(|e| format!("Cannot open image: {e}"))?;
    let (w, h) = img.dimensions();
    let (new_w, new_h) = compute_target_size(w, h, max_size);

    let resized = if w > max_size || h > max_size {
        img.resize(new_w, new_h, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let rgb = resized.to_rgb8();
    rgb.save_with_format(dst_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Cannot save thumbnail: {e}"))?;

    Ok(())
}

/// Compute target dimensions preserving aspect ratio, capped at max_size.
fn compute_target_size(w: u32, h: u32, max_size: u32) -> (u32, u32) {
    if w <= max_size && h <= max_size {
        return (w, h);
    }
    let ratio = max_size as f32 / w.max(h) as f32;
    let new_w = (w as f32 * ratio).round().max(1.0) as u32;
    let new_h = (h as f32 * ratio).round().max(1.0) as u32;
    (new_w, new_h)
}

/// Fast bilinear resize on raw RGB8 pixels.
fn resize_bilinear(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w as usize) * (dst_h as usize) * 3];
    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;

    for y in 0..dst_h {
        let src_y = (y as f32 * y_ratio).min(src_h as f32 - 1.0);
        let y_low = src_y.floor() as u32;
        let y_frac = src_y - y_low as f32;
        let y_high = (y_low + 1).min(src_h - 1);

        for x in 0..dst_w {
            let src_x = (x as f32 * x_ratio).min(src_w as f32 - 1.0);
            let x_low = src_x.floor() as u32;
            let x_frac = src_x - x_low as f32;
            let x_high = (x_low + 1).min(src_w - 1);

            for c in 0..3usize {
                let v00 = src[((y_low as usize * src_w as usize + x_low as usize) * 3) + c] as f32;
                let v10 = src[((y_low as usize * src_w as usize + x_high as usize) * 3) + c] as f32;
                let v01 = src[((y_high as usize * src_w as usize + x_low as usize) * 3) + c] as f32;
                let v11 = src[((y_high as usize * src_w as usize + x_high as usize) * 3) + c] as f32;

                let val = v00 * (1.0 - x_frac) * (1.0 - y_frac)
                    + v10 * x_frac * (1.0 - y_frac)
                    + v01 * (1.0 - x_frac) * y_frac
                    + v11 * x_frac * y_frac;

                dst[((y as usize * dst_w as usize + x as usize) * 3) + c] = val.round().min(255.0) as u8;
            }
        }
    }
    dst
}
