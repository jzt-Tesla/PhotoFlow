use serde::Serialize;
use std::sync::Arc;
use tauri::Emitter;

use crate::db::{AppSettings, Database, Photo, ScanDirectory, Tag, TimelineGroup};
use crate::scanner;
use crate::thumbnail;
use crate::{add_allowed_root, AppDataDir};

/// Event payload emitted during scanning.
#[derive(Clone, Serialize)]
pub struct ScanProgress {
    pub found: u64,
    pub indexed: u64,
    pub errors: u64,
}

/// Result returned after a scan completes.
#[derive(Clone, Serialize)]
pub struct ScanResult {
    pub message: String,
    pub found: u64,
    pub indexed: u64,
    pub errors: u64,
    pub cleanup_removed: u64,
}

// ────────────── Internal scan helper ──────────────

fn do_scan(
    app: &tauri::AppHandle,
    db: &Database,
    data_dir: &std::path::Path,
    dir_str: &str,
) -> Result<ScanResult, String> {
    let thumb_dir = data_dir.join("thumbs");
    let app_handle = app.clone();
    let app_handle_photo = app.clone();
    let cleanup_removed = db.cleanup_orphans(&thumb_dir) as u64;

    let stats = scanner::scan_directory_incremental(
        db,
        dir_str,
        &thumb_dir,
        // Progress callback
        |found, indexed, errors| {
            let _ = app_handle.emit(
                "scan-progress",
                ScanProgress {
                    found,
                    indexed,
                    errors,
                },
            );
        },
        // Photo callback — emit EACH photo immediately, no DB round-trip
        |photo| {
            let _ = app_handle_photo.emit("photo-stream", &photo);
        },
    );

    Ok(ScanResult {
        message: format!(
            "扫描 {} 个文件，新增 {}，更新 {}，删除 {}，错误 {}",
            stats.found, stats.indexed, stats.updated, stats.removed, stats.errors
        ),
        found: stats.found,
        indexed: stats.indexed + stats.updated,
        errors: stats.errors,
        cleanup_removed: stats.removed + cleanup_removed,
    })
}

// ────────────── Directory scanning commands ──────────────

#[tauri::command]
pub async fn select_and_scan_directory(
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    data_dir: tauri::State<'_, AppDataDir>,
) -> Result<ScanResult, String> {
    let dir = rfd::FileDialog::new()
        .set_title("选择照片目录")
        .pick_folder()
        .ok_or("No directory selected")?;

    let dir_str = dir.to_string_lossy().to_string();
    db.add_scan_directory(&dir_str)
        .map_err(|e| format!("Cannot save directory: {e}"))?;
    add_allowed_root(dir.clone());

    let app_handle = app.clone();
    let db_clone = Arc::clone(&db);
    let data_path = data_dir.0.clone();

    tokio::task::spawn_blocking(move || do_scan(&app_handle, &db_clone, &data_path, &dir_str))
        .await
        .map_err(|e| format!("Scan task failed: {e}"))?
}

#[tauri::command]
pub async fn rescan_all_directories(
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    data_dir: tauri::State<'_, AppDataDir>,
) -> Result<ScanResult, String> {
    let dirs = db
        .list_scan_directories()
        .map_err(|e| format!("Cannot read directories: {e}"))?;

    if dirs.is_empty() {
        return Err("没有已添加的目录".to_string());
    }

    let mut total_found = 0u64;
    let mut total_indexed = 0u64;
    let mut total_errors = 0u64;
    let mut total_removed = 0u64;

    for dir_entry in &dirs {
        let dir_str = dir_entry.path.clone();
        let app_handle = app.clone();
        let db_clone = Arc::clone(&db);
        let data_path = data_dir.0.clone();

        let result = tokio::task::spawn_blocking(move || {
            do_scan(&app_handle, &db_clone, &data_path, &dir_str)
        })
        .await
        .map_err(|e| format!("Scan task failed: {e}"))??;

        total_found += result.found;
        total_indexed += result.indexed;
        total_errors += result.errors;
        total_removed += result.cleanup_removed;
    }

    Ok(ScanResult {
        message: format!(
            "扫描 {} 个目录，共 {} 个文件，{} 已索引，{} 错误",
            dirs.len(),
            total_found,
            total_indexed,
            total_errors
        ),
        found: total_found,
        indexed: total_indexed,
        errors: total_errors,
        cleanup_removed: total_removed,
    })
}

#[tauri::command]
pub async fn rescan_directory_by_id(
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    data_dir: tauri::State<'_, AppDataDir>,
    dir_id: i64,
) -> Result<ScanResult, String> {
    let dirs = db
        .list_scan_directories()
        .map_err(|e| format!("Cannot read directories: {e}"))?;
    let dir = dirs
        .iter()
        .find(|d| d.id == dir_id)
        .ok_or("目录不存在")?;

    let dir_str = dir.path.clone();
    let app_handle = app.clone();
    let db_clone = Arc::clone(&db);
    let data_path = data_dir.0.clone();

    tokio::task::spawn_blocking(move || do_scan(&app_handle, &db_clone, &data_path, &dir_str))
        .await
        .map_err(|e| format!("Scan task failed: {e}"))?
}

// ────────────── Photo loading commands ──────────────

#[tauri::command]
pub fn load_photos(
    db: tauri::State<'_, Arc<Database>>,
    offset: i64,
    limit: i64,
) -> Result<Vec<Photo>, String> {
    db.load_photos(offset, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_photo_count(db: tauri::State<'_, Arc<Database>>) -> Result<i64, String> {
    db.photo_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_photos_filtered(
    db: tauri::State<'_, Arc<Database>>,
    filter_json: String,
    offset: i64,
    limit: i64,
) -> Result<Vec<Photo>, String> {
    let filter: crate::db::PhotoFilter = serde_json::from_str(&filter_json)
        .map_err(|e| format!("Invalid filter: {e}"))?;
    db.load_photos_filtered(&filter, offset, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn photo_count_filtered(
    db: tauri::State<'_, Arc<Database>>,
    filter_json: String,
) -> Result<i64, String> {
    let filter: crate::db::PhotoFilter = serde_json::from_str(&filter_json)
        .map_err(|e| format!("Invalid filter: {e}"))?;
    db.photo_count_filtered(&filter).map_err(|e| e.to_string())
}

// ────────────── Favorites ──────────────

#[tauri::command]
pub fn toggle_favorite(
    db: tauri::State<'_, Arc<Database>>,
    photo_id: i64,
) -> Result<bool, String> {
    db.toggle_favorite(photo_id).map_err(|e| e.to_string())
}

// ────────────── Tags ──────────────

#[tauri::command]
pub fn create_tag(
    db: tauri::State<'_, Arc<Database>>,
    name: String,
    color: String,
) -> Result<i64, String> {
    db.create_tag(&name, &color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_tag(db: tauri::State<'_, Arc<Database>>, tag_id: i64) -> Result<(), String> {
    db.delete_tag(tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_tag(
    db: tauri::State<'_, Arc<Database>>,
    tag_id: i64,
    new_name: String,
) -> Result<(), String> {
    db.rename_tag(tag_id, &new_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_tag_color(
    db: tauri::State<'_, Arc<Database>>,
    tag_id: i64,
    color: String,
) -> Result<(), String> {
    db.update_tag_color(tag_id, &color)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_tags(db: tauri::State<'_, Arc<Database>>) -> Result<Vec<Tag>, String> {
    db.list_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_photo_tag(
    db: tauri::State<'_, Arc<Database>>,
    photo_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    db.add_photo_tag(photo_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_photo_tag(
    db: tauri::State<'_, Arc<Database>>,
    photo_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    db.remove_photo_tag(photo_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_photo_tags(
    db: tauri::State<'_, Arc<Database>>,
    photo_id: i64,
) -> Result<Vec<Tag>, String> {
    db.get_photo_tags(photo_id).map_err(|e| e.to_string())
}

// ────────────── Timeline ──────────────

#[tauri::command]
pub fn load_timeline_groups(
    db: tauri::State<'_, Arc<Database>>,
) -> Result<Vec<TimelineGroup>, String> {
    db.load_timeline_groups().map_err(|e| e.to_string())
}

// ────────────── Multi-directory ──────────────

#[tauri::command]
pub fn add_scan_directory(
    db: tauri::State<'_, Arc<Database>>,
    path: String,
) -> Result<i64, String> {
    let id = db
        .add_scan_directory(&path)
        .map_err(|e| format!("Cannot add directory: {e}"))?;
    add_allowed_root(std::path::PathBuf::from(&path));
    Ok(id)
}

#[tauri::command]
pub fn remove_scan_directory(
    db: tauri::State<'_, Arc<Database>>,
    dir_id: i64,
) -> Result<(), String> {
    // Look up the path before deleting so we can remove from ALLOWED_ROOTS
    let dirs = db.list_scan_directories().map_err(|e| e.to_string())?;
    let dir_path = dirs
        .iter()
        .find(|d| d.id == dir_id)
        .map(|d| std::path::PathBuf::from(&d.path));

    db.remove_scan_directory(dir_id)
        .map_err(|e| e.to_string())?;

    if let Some(path) = dir_path {
        crate::remove_allowed_root(&path);
    }
    Ok(())
}

#[tauri::command]
pub fn list_scan_directories(
    db: tauri::State<'_, Arc<Database>>,
) -> Result<Vec<ScanDirectory>, String> {
    db.list_scan_directories().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_and_add_directory(
    db: tauri::State<'_, Arc<Database>>,
) -> Result<i64, String> {
    let dir = rfd::FileDialog::new()
        .set_title("选择照片目录")
        .pick_folder()
        .ok_or("未选择目录")?;
    let dir_str = dir.to_string_lossy().to_string();
    let id = db
        .add_scan_directory(&dir_str)
        .map_err(|e| format!("Cannot add directory: {e}"))?;
    add_allowed_root(dir);
    Ok(id)
}

// ────────────── Settings ──────────────

#[tauri::command]
pub fn get_settings(db: tauri::State<'_, Arc<Database>>) -> Result<AppSettings, String> {
    Ok(db.get_app_settings())
}

#[tauri::command]
pub fn update_settings(
    db: tauri::State<'_, Arc<Database>>,
    settings: AppSettings,
) -> Result<(), String> {
    if !(50..=2000).contains(&settings.thumbnail_size) {
        return Err("thumbnail_size must be between 50 and 2000".into());
    }
    if !(1..=100).contains(&settings.thumbnail_quality) {
        return Err("thumbnail_quality must be between 1 and 100".into());
    }
    db.save_app_settings(&settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_thumbnail_cache(
    db: tauri::State<'_, Arc<Database>>,
    data_dir: tauri::State<'_, AppDataDir>,
) -> Result<u64, String> {
    let thumb_dir = data_dir.0.join("thumbs");
    let removed = db.cleanup_orphans(&thumb_dir);
    Ok(removed as u64)
}

#[tauri::command]
pub fn get_app_info(
    db: tauri::State<'_, Arc<Database>>,
    data_dir: tauri::State<'_, AppDataDir>,
) -> Result<AppInfo, String> {
    let count = db.photo_count().unwrap_or(0);
    let db_path = data_dir.0.join("photoflow.db");
    let db_size = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let thumb_dir = data_dir.0.join("thumbs");
    let thumb_count = std::fs::read_dir(&thumb_dir)
        .map(|entries| entries.filter_map(|e| e.ok()).count() as i64)
        .unwrap_or(0);
    let dirs = db.list_scan_directories().unwrap_or_default();

    Ok(AppInfo {
        photo_count: count,
        db_size,
        thumbnail_count: thumb_count,
        scan_directories: dirs,
        data_dir: data_dir.0.to_string_lossy().to_string(),
    })
}

#[derive(Clone, Serialize)]
pub struct AppInfo {
    pub photo_count: i64,
    pub db_size: u64,
    pub thumbnail_count: i64,
    pub scan_directories: Vec<ScanDirectory>,
    pub data_dir: String,
}

/// Repair missing thumbnails: regenerate for photos whose thumbnail file is missing.
#[tauri::command]
pub fn repair_thumbnails(
    db: tauri::State<'_, Arc<Database>>,
    data_dir: tauri::State<'_, AppDataDir>,
) -> Result<u64, String> {
    let thumb_dir = data_dir.0.join("thumbs");
    std::fs::create_dir_all(&thumb_dir).ok();
    let settings = db.get_app_settings();
    let thumb_size = settings.thumbnail_size as u32;

    let photos = db.get_all_photo_paths_and_thumbs().map_err(|e| e.to_string())?;
    let mut repaired = 0u64;

    for (photo_path, old_thumb_path) in &photos {
        if std::path::Path::new(old_thumb_path).exists() {
            continue;
        }
        let photo = std::path::Path::new(photo_path);
        if !photo.exists() {
            continue;
        }
        let hash = thumbnail::hash_path(photo_path);
        let new_thumb_path = thumb_dir.join(format!("{}.jpg", hash));
        if thumbnail::generate_thumbnail(photo, &new_thumb_path, thumb_size).is_ok() {
            let _ = db.update_thumbnail_path(photo_path, &new_thumb_path.to_string_lossy());
            repaired += 1;
        }
    }
    Ok(repaired)
}
