use chrono::NaiveDateTime;
use exif::{In, Tag};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use walkdir::WalkDir;

use crate::db::Database;
use crate::thumbnail;

/// Supported image file extensions (lowercase, without dot).
const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Batch size for database inserts — balances lock contention vs memory.
const BATCH_SIZE: usize = 50;

/// Check if a file extension indicates a supported image type.
fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Try to read the EXIF DateTimeOriginal from a JPEG file.
fn read_exif_date(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut bufreader = BufReader::new(file);
    let exif = exif::Reader::new()
        .read_from_container(&mut bufreader)
        .ok()?;

    let field = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::DateTime, In::PRIMARY))?;

    let value = match field.value {
        exif::Value::Ascii(ref v) if !v.is_empty() => {
            String::from_utf8_lossy(&v[0]).trim().to_string()
        }
        _ => return None,
    };

    NaiveDateTime::parse_from_str(&value, "%Y:%m:%d %H:%M:%S")
        .ok()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// Read image dimensions (works for all supported formats).
fn read_dimensions(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}

/// Safely convert u32 dimension to i32, capping at i32::MAX.
fn dim_to_i32(val: u32) -> i32 {
    u32::min(val, i32::MAX as u32) as i32
}

/// Get a file's last-modified time as a fallback date string.
fn file_modified_time(path: &Path) -> String {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            let datetime: chrono::DateTime<chrono::Local> = t.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|_| "1970-01-01 00:00:00".to_string())
}

/// Scan a directory recursively, index all found photos into the database,
/// and generate thumbnails. Calls `on_progress` with (found, indexed, errors) counts.
/// Returns (found, indexed, errors).
/// Uses batched inserts for reduced lock contention.
pub fn scan_directory<F>(
    db: &Database,
    dir: &str,
    thumb_dir: &Path,
    on_progress: F,
) -> (u64, u64, u64)
where
    F: Fn(u64, u64, u64),
{
    std::fs::create_dir_all(thumb_dir).ok();

    let mut found = 0u64;
    let mut indexed = 0u64;
    let mut errors = 0u64;
    let mut batch: Vec<(String, String, String, i32, i32, String)> = Vec::with_capacity(BATCH_SIZE);

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| {
        match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("WalkDir error: {err}");
                None
            }
        }
    }) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_supported_image(path) {
            continue;
        }
        found += 1;

        let path_str = path.to_string_lossy().to_string();
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        // Read dimensions
        let (width, height) = match read_dimensions(path) {
            Some(dims) => dims,
            None => {
                errors += 1;
                on_progress(found, indexed, errors);
                continue;
            }
        };

        // Read EXIF date or fall back to file modified time
        let created_time = read_exif_date(path).unwrap_or_else(|| file_modified_time(path));

        // Generate thumbnail path using a hash of the file path
        let hash = thumbnail::hash_path(&path_str);
        let thumb_filename = format!("{}.jpg", hash);
        let thumb_path = thumb_dir.join(&thumb_filename);
        let thumb_path_str = thumb_path.to_string_lossy().to_string();

        // Generate thumbnail if it doesn't exist
        if !thumb_path.exists() && thumbnail::generate_thumbnail(path, &thumb_path, 200).is_err() {
            errors += 1;
            on_progress(found, indexed, errors);
            continue;
        }

        // Add to batch
        batch.push((
            path_str,
            filename,
            created_time,
            dim_to_i32(width),
            dim_to_i32(height),
            thumb_path_str,
        ));
        indexed += 1;

        // Flush batch when full
        if batch.len() >= BATCH_SIZE {
            if let Err(e) = db.insert_photos_batch(&batch) {
                eprintln!("Batch insert error: {e}");
                errors += batch.len() as u64;
                indexed -= batch.len() as u64;
            }
            batch.clear();
        }

        on_progress(found, indexed, errors);
    }

    // Flush remaining photos
    if !batch.is_empty() {
        if let Err(e) = db.insert_photos_batch(&batch) {
            eprintln!("Batch insert error: {e}");
            errors += batch.len() as u64;
            indexed -= batch.len() as u64;
        }
    }

    (found, indexed, errors)
}
