use chrono::NaiveDateTime;
use exif::{In, Tag, Value};
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, File, Metadata};
use std::io::BufReader;
use std::path::Path;
use walkdir::WalkDir;

use crate::db::Database;
use crate::thumbnail;

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const DB_BATCH_SIZE: usize = 100;

pub struct ExifData {
    pub taken_time: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

pub struct ScanStats {
    pub found: u64,
    pub indexed: u64,
    pub updated: u64,
    pub removed: u64,
    pub errors: u64,
}

/// Lightweight photo struct for streaming to frontend.
/// Built in-memory — no DB round-trip needed.
#[derive(Clone, Serialize)]
pub struct PhotoStream {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub created_time: String,
    pub width: i32,
    pub height: i32,
    pub thumbnail_path: String,
    pub file_size: i64,
    pub modified_time: String,
    pub taken_time: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub favorite: bool,
    pub tags: Vec<String>,
}

struct FileMeta {
    path: String,
    size: i64,
    mtime: String,
}

struct UpsertEntry {
    path: String,
    filename: String,
    created_time: String,
    width: i32,
    height: i32,
    thumb_path: String,
    file_size: i64,
    modified_time: String,
    exif: ExifData,
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn stat_file(path: &Path) -> Option<(Metadata, i64, String)> {
    let meta = fs::metadata(path).ok()?;
    let size = meta.len() as i64;
    let mtime = meta
        .modified()
        .map(|t| {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|_| "1970-01-01 00:00:00".to_string());
    Some((meta, size, mtime))
}

fn dim_to_i32(val: u32) -> i32 {
    u32::min(val, i32::MAX as u32) as i32
}

fn parse_gps_coord(values: &[Value], ref_val: &str) -> Option<f64> {
    if values.len() < 3 {
        return None;
    }
    let degrees = match &values[0] {
        Value::Rational(r) if !r.is_empty() => r[0].to_f64(),
        _ => return None,
    };
    let minutes = match &values[1] {
        Value::Rational(r) if !r.is_empty() => r[0].to_f64(),
        _ => return None,
    };
    let seconds = match &values[2] {
        Value::Rational(r) if !r.is_empty() => r[0].to_f64(),
        _ => return None,
    };
    let mut dd = degrees + minutes / 60.0 + seconds / 3600.0;
    if ref_val == "S" || ref_val == "W" {
        dd = -dd;
    }
    Some(dd)
}

fn read_exif_metadata(path: &Path) -> ExifData {
    let mut result = ExifData {
        taken_time: None,
        camera_model: None,
        lens_model: None,
        latitude: None,
        longitude: None,
    };
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return result,
    };
    let mut buf = BufReader::new(file);
    let exif = match exif::Reader::new().read_from_container(&mut buf) {
        Ok(e) => e,
        Err(_) => return result,
    };

    if let Some(field) = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::DateTime, In::PRIMARY))
    {
        if let Value::Ascii(ref v) = field.value {
            if !v.is_empty() {
                let val = String::from_utf8_lossy(&v[0]).trim().to_string();
                result.taken_time = NaiveDateTime::parse_from_str(&val, "%Y:%m:%d %H:%M:%S")
                    .ok()
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
            }
        }
    }
    if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
        if let Value::Ascii(ref v) = field.value {
            if !v.is_empty() {
                result.camera_model = Some(String::from_utf8_lossy(&v[0]).trim().to_string());
            }
        }
    }
    if let Some(field) = exif
        .get_field(Tag::LensModel, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::LensMake, In::PRIMARY))
    {
        if let Value::Ascii(ref v) = field.value {
            if !v.is_empty() {
                result.lens_model = Some(String::from_utf8_lossy(&v[0]).trim().to_string());
            }
        }
    }
    if let Some(lat_field) = exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
        if let Some(lat_ref) = exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY) {
            let ref_str = match &lat_ref.value {
                Value::Ascii(v) if !v.is_empty() => {
                    String::from_utf8_lossy(&v[0]).trim().to_string()
                }
                _ => "N".to_string(),
            };
            if let Value::Rational(ref coords) = lat_field.value {
                result.latitude = parse_gps_coord(&[Value::Rational(coords.clone())], &ref_str);
            }
        }
    }
    if let Some(lon_field) = exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
        if let Some(lon_ref) = exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY) {
            let ref_str = match &lon_ref.value {
                Value::Ascii(v) if !v.is_empty() => {
                    String::from_utf8_lossy(&v[0]).trim().to_string()
                }
                _ => "E".to_string(),
            };
            if let Value::Rational(ref coords) = lon_field.value {
                result.longitude = parse_gps_coord(&[Value::Rational(coords.clone())], &ref_str);
            }
        }
    }
    result
}

/// Streaming incremental scan.
///
/// - `on_progress(found, indexed, errors)` — fires after each file
/// - `on_photo(photo)` — fires IMMEDIATELY after each photo is processed
///   (thumbnail generated, metadata extracted). No DB round-trip.
/// - DB writes batched every 100 photos for throughput.
pub fn scan_directory_incremental<F, P>(
    db: &Database,
    dir: &str,
    thumb_dir: &Path,
    on_progress: F,
    on_photo: P,
) -> ScanStats
where
    F: Fn(u64, u64, u64),
    P: Fn(PhotoStream),
{
    std::fs::create_dir_all(thumb_dir).ok();

    let settings = db.get_app_settings();
    let thumb_size = settings.thumbnail_size as u32;

    let mut indexed = 0u64;
    let mut updated = 0u64;
    let mut errors = 0u64;
    let mut next_id: i64 = -1; // Negative IDs for temp photos (DB IDs are always positive)

    // Phase 1: Collect files
    let mut fs_files: Vec<FileMeta> = Vec::new();
    let mut fs_paths: HashSet<String> = HashSet::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| match e {
        Ok(entry) => Some(entry),
        Err(err) => {
            eprintln!("WalkDir error: {err}");
            None
        }
    }) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_supported_image(path) {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        if let Some((_meta, size, mtime)) = stat_file(path) {
            fs_files.push(FileMeta {
                path: path_str.clone(),
                size,
                mtime,
            });
            fs_paths.insert(path_str);
        }
    }

    let found = fs_files.len() as u64;

    // Phase 2: Batch check
    let check_input: Vec<(String, i64, String)> = fs_files
        .iter()
        .map(|f| (f.path.clone(), f.size, f.mtime.clone()))
        .collect();
    let check_result = match db.batch_check_existing(&check_input) {
        Ok(r) => r,
        Err(_) => {
            return ScanStats {
                found,
                indexed,
                updated,
                removed: 0,
                errors: found,
            };
        }
    };

    // Phase 3: Remove deleted files
    let removed = db.delete_missing_photos(&fs_paths, thumb_dir).unwrap_or(0) as u64;

    // Phase 4: Process new + modified — emit each photo immediately
    let to_process: Vec<&str> = check_result
        .new_paths
        .iter()
        .chain(check_result.modified_paths.iter())
        .map(|s| s.as_str())
        .collect();

    let mut db_batch: Vec<UpsertEntry> = Vec::with_capacity(DB_BATCH_SIZE);

    for path_str in &to_process {
        let path = Path::new(path_str);
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        let file_meta = fs_files.iter().find(|f| f.path.as_str() == *path_str);
        let (fsize, fmtime) = match file_meta {
            Some(m) => (m.size, m.mtime.clone()),
            None => {
                errors += 1;
                continue;
            }
        };

        let (width, height) = match image::image_dimensions(path) {
            Ok(dims) => (dim_to_i32(dims.0), dim_to_i32(dims.1)),
            Err(_) => {
                errors += 1;
                on_progress(found, indexed + updated, errors);
                continue;
            }
        };

        let exif = read_exif_metadata(path);
        let created_time = exif.taken_time.clone().unwrap_or_else(|| fmtime.clone());

        let hash = thumbnail::hash_path(path_str);
        let thumb_filename = format!("{}.jpg", hash);
        let thumb_path = thumb_dir.join(&thumb_filename);
        let thumb_path_str = thumb_path.to_string_lossy().to_string();

        if !thumb_path.exists()
            && thumbnail::generate_thumbnail(path, &thumb_path, thumb_size).is_err()
        {
            errors += 1;
            on_progress(found, indexed + updated, errors);
            continue;
        }

        // ── Stream photo to frontend IMMEDIATELY ──
        next_id -= 1;
        let photo_stream = PhotoStream {
            id: next_id,
            path: path_str.to_string(),
            filename: filename.clone(),
            created_time: created_time.clone(),
            width,
            height,
            thumbnail_path: thumb_path_str.clone(),
            file_size: fsize,
            modified_time: fmtime.clone(),
            taken_time: exif.taken_time.clone(),
            camera_model: exif.camera_model.clone(),
            lens_model: exif.lens_model.clone(),
            latitude: exif.latitude,
            longitude: exif.longitude,
            favorite: false,
            tags: Vec::new(),
        };
        on_photo(photo_stream);

        // ── Buffer for DB batch write ──
        db_batch.push(UpsertEntry {
            path: path_str.to_string(),
            filename,
            created_time,
            width,
            height,
            thumb_path: thumb_path_str,
            file_size: fsize,
            modified_time: fmtime,
            exif: ExifData {
                taken_time: exif.taken_time,
                camera_model: exif.camera_model,
                lens_model: exif.lens_model,
                latitude: exif.latitude,
                longitude: exif.longitude,
            },
        });

        if check_result.new_paths.iter().any(|p| p.as_str() == *path_str) {
            indexed += 1;
        } else {
            updated += 1;
        }

        // Flush DB batch every 100
        if db_batch.len() >= DB_BATCH_SIZE {
            if let Err(e) = flush_db_batch(db, &db_batch) {
                eprintln!("DB batch write failed: {e}");
                errors += db_batch.len() as u64;
            }
            db_batch.clear();
        }

        on_progress(found, indexed + updated, errors);
    }

    // Flush remaining DB batch
    if !db_batch.is_empty() {
        if let Err(e) = flush_db_batch(db, &db_batch) {
            eprintln!("DB batch write failed: {e}");
            errors += db_batch.len() as u64;
        }
    }

    on_progress(found, indexed + updated, errors);

    ScanStats {
        found,
        indexed,
        updated,
        removed,
        errors,
    }
}

fn flush_db_batch(db: &Database, batch: &[UpsertEntry]) -> Result<(), String> {
    let tuples: Vec<(
        String, String, String, i32, i32, String, i64, String,
        Option<String>, Option<String>, Option<String>, Option<f64>, Option<f64>,
    )> = batch
        .iter()
        .map(|e| {
            (
                e.path.clone(),
                e.filename.clone(),
                e.created_time.clone(),
                e.width,
                e.height,
                e.thumb_path.clone(),
                e.file_size,
                e.modified_time.clone(),
                e.exif.taken_time.clone(),
                e.exif.camera_model.clone(),
                e.exif.lens_model.clone(),
                e.exif.latitude,
                e.exif.longitude,
            )
        })
        .collect();

    db.upsert_photos_batch(&tuples).map_err(|e| e.to_string())
}
