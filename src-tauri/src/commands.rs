use serde::Serialize;
use std::sync::Arc;
use tauri::Emitter;

use crate::db::{Database, Photo};
use crate::scanner;
use crate::{add_allowed_root, AppDataDir};

/// Event payload emitted during scanning.
#[derive(Clone, Serialize)]
pub struct ScanProgress {
    found: u64,
    indexed: u64,
    errors: u64,
}

/// Result returned after a scan completes.
#[derive(Clone, Serialize)]
pub struct ScanResult {
    message: String,
    found: u64,
    indexed: u64,
    errors: u64,
    cleanup_removed: u64,
}

/// Internal: run a scan given a directory path, emitting progress events.
fn do_scan(
    app: &tauri::AppHandle,
    db: &Database,
    data_dir: &std::path::Path,
    dir_str: &str,
) -> Result<ScanResult, String> {
    let thumb_dir = data_dir.join("thumbs");

    let app_handle = app.clone();

    // Cleanup orphaned records first
    let cleanup_removed = db.cleanup_orphans(&thumb_dir) as u64;

    let (found, indexed, errors) =
        scanner::scan_directory(db, dir_str, &thumb_dir, |found, indexed, errors| {
            let _ = app_handle.emit(
                "scan-progress",
                ScanProgress {
                    found,
                    indexed,
                    errors,
                },
            );
        });

    Ok(ScanResult {
        message: format!(
            "扫描 {} 个文件，索引 {} 张照片，{} 个错误",
            found, indexed, errors
        ),
        found,
        indexed,
        errors,
        cleanup_removed,
    })
}

/// Open a directory picker dialog, scan the selected directory for photos,
/// and index them into the database. Emits "scan-progress" events during scan.
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

    // Save the selected directory for future rescans
    db.set_setting("scan_dir", &dir_str)
        .map_err(|e| format!("Cannot save setting: {e}"))?;

    // Allow the /photo/ endpoint to serve files from this directory
    add_allowed_root(dir.clone());

    let app_handle = app.clone();
    let db_clone = Arc::clone(&db);
    let data_path = data_dir.0.clone();

    tokio::task::spawn_blocking(move || do_scan(&app_handle, &db_clone, &data_path, &dir_str))
        .await
        .map_err(|e| format!("Scan task failed: {e}"))?
}

/// Re-scan the previously selected directory.
#[tauri::command]
pub async fn rescan_directory(
    app: tauri::AppHandle,
    db: tauri::State<'_, Arc<Database>>,
    data_dir: tauri::State<'_, AppDataDir>,
) -> Result<ScanResult, String> {
    let dir_str = db
        .get_setting("scan_dir")
        .map_err(|e| format!("Cannot read setting: {e}"))?
        .ok_or("No directory previously selected")?;

    let app_handle = app.clone();
    let db_clone = Arc::clone(&db);
    let data_path = data_dir.0.clone();

    tokio::task::spawn_blocking(move || do_scan(&app_handle, &db_clone, &data_path, &dir_str))
        .await
        .map_err(|e| format!("Scan task failed: {e}"))?
}

/// Load a page of photos ordered by created_time DESC.
/// Offset and limit are validated and clamped to safe ranges.
#[tauri::command]
pub fn load_photos(
    db: tauri::State<'_, Arc<Database>>,
    offset: i64,
    limit: i64,
) -> Result<Vec<Photo>, String> {
    db.load_photos(offset, limit).map_err(|e| e.to_string())
}

/// Return the total number of indexed photos.
#[tauri::command]
pub fn get_photo_count(db: tauri::State<'_, Arc<Database>>) -> Result<i64, String> {
    db.photo_count().map_err(|e| e.to_string())
}
