pub mod commands;
pub mod db;
pub mod scanner;
pub mod thumbnail;

use base64::Engine;
use db::Database;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};
use tauri::Manager;

/// Global data directory, set once during setup.
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Allowed root directories for the /photo/ endpoint.
static ALLOWED_ROOTS: OnceLock<RwLock<Vec<PathBuf>>> = OnceLock::new();

/// Wrapper for the app data directory path, stored as Tauri state.
#[derive(Clone)]
pub struct AppDataDir(pub PathBuf);

/// Get the global data directory (available after setup).
pub fn data_dir() -> &'static PathBuf {
    DATA_DIR
        .get()
        .expect("DATA_DIR not initialized — app setup has not run")
}

/// Add a directory to the allowed photo roots.
pub fn add_allowed_root(path: PathBuf) {
    if let Some(lock) = ALLOWED_ROOTS.get() {
        if let Ok(mut roots) = lock.write() {
            if !roots.iter().any(|r| r == &path) {
                roots.push(path);
            }
        }
    }
}

/// Remove a directory from the allowed photo roots.
pub fn remove_allowed_root(path: &PathBuf) {
    if let Some(lock) = ALLOWED_ROOTS.get() {
        if let Ok(mut roots) = lock.write() {
            roots.retain(|r| r != path);
        }
    }
}

fn get_allowed_roots() -> Vec<PathBuf> {
    ALLOWED_ROOTS
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|roots| roots.clone())
        .unwrap_or_default()
}

/// Extract the path portion from a URI string.
fn uri_path(uri: &str) -> &str {
    if let Some(rest) = uri
        .strip_prefix("http://")
        .or_else(|| uri.strip_prefix("https://"))
    {
        if let Some(slash_pos) = rest.find('/') {
            return &rest[slash_pos..];
        }
        return "/";
    }
    if let Some(idx) = uri.find(":/") {
        let after_scheme = &uri[idx + 2..];
        if after_scheme.starts_with('/') {
            let host_and_path = &after_scheme[1..];
            if let Some(host_end) = host_and_path.find('/') {
                return &host_and_path[host_end..];
            }
            return "/";
        }
        return &uri[idx + 1..];
    }
    uri
}

/// Validate that a decoded photo path is within one of the allowed root directories.
/// NOTE: canonicalize() follows symlinks. Defense-in-depth: detect_image_mime()
/// prevents serving non-image files regardless of path.
fn is_allowed_photo_path(file_path: &str, allowed_roots: &[PathBuf]) -> bool {
    let path = std::path::Path::new(file_path);
    if file_path.contains("..") {
        return false;
    }
    if !path.is_absolute() {
        return false;
    }
    if let Ok(canon) = path.canonicalize() {
        return allowed_roots
            .iter()
            .any(|root| root.canonicalize().ok().map_or(false, |r| canon.starts_with(&r)));
    }
    false
}

/// Detect MIME type from file magic bytes. Returns None for non-image files.
fn detect_image_mime(data: &[u8]) -> Option<&'static str> {
    if data.len() >= 3 && data[..3] == [0xFF, 0xD8, 0xFF] {
        return Some("image/jpeg");
    }
    if data.len() >= 8 && data[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some("image/png");
    }
    if data.len() >= 4 && &data[..4] == b"RIFF" {
        return Some("image/webp");
    }
    None
}

/// Build and run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // NOTE: tauri-plugin-fs removed — all file I/O is handled by Rust backend directly.
        // Frontend does not use fs plugin IPC.
        .register_uri_scheme_protocol("photoflow", |_app, request| {
            // H2 fix: graceful handling if DATA_DIR not yet initialized
            let data_dir = match DATA_DIR.get() {
                Some(dir) => dir,
                None => {
                    return http::Response::builder()
                        .status(503)
                        .body(b"App not ready".to_vec())
                        .unwrap();
                }
            };
            let thumb_dir = data_dir.join("thumbs");
            let allowed_roots = get_allowed_roots();

            let uri_str = request.uri().to_string();
            let path = uri_path(&uri_str);

            // Route: /thumb/{hash} → serve thumbnail JPEG
            if let Some(hash) = path.strip_prefix("/thumb/") {
                if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    return http::Response::builder()
                        .status(400)
                        .body(Vec::new())
                        .unwrap();
                }
                let thumb_path = thumb_dir.join(format!("{}.jpg", hash));
                return match std::fs::read(&thumb_path) {
                    Ok(bytes) => http::Response::builder()
                        .status(200)
                        .header("Content-Type", "image/jpeg")
                        .header("Cache-Control", "max-age=86400")
                        .body(bytes)
                        .unwrap(),
                    Err(_) => http::Response::builder()
                        .status(404)
                        .body(Vec::new())
                        .unwrap(),
                };
            }

            // Route: /photo/{base64_encoded_path} → serve original photo
            if let Some(encoded) = path.strip_prefix("/photo/") {
                return match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded) {
                    Ok(bytes) => {
                        let file_path = String::from_utf8_lossy(&bytes).to_string();

                        if !is_allowed_photo_path(&file_path, &allowed_roots) {
                            return http::Response::builder()
                                .status(403)
                                .body(Vec::new())
                                .unwrap();
                        }

                        match std::fs::read(&file_path) {
                            Ok(data) => {
                                let mime = match detect_image_mime(&data) {
                                    Some(m) => m,
                                    None => {
                                        return http::Response::builder()
                                            .status(415)
                                            .body(Vec::new())
                                            .unwrap();
                                    }
                                };
                                http::Response::builder()
                                    .status(200)
                                    .header("Content-Type", mime)
                                    .body(data)
                                    .unwrap()
                            }
                            Err(_) => http::Response::builder()
                                .status(404)
                                .body(Vec::new())
                                .unwrap(),
                        }
                    }
                    Err(_) => http::Response::builder()
                        .status(400)
                        .body(Vec::new())
                        .unwrap(),
                };
            }

            http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap()
        })
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
            std::fs::create_dir_all(&app_data)
                .map_err(|e| format!("Cannot create app data directory: {e}"))?;

            let thumb_dir = app_data.join("thumbs");
            std::fs::create_dir_all(&thumb_dir)
                .map_err(|e| format!("Cannot create thumbnail directory: {e}"))?;

            let db_path = app_data.join("photoflow.db");
            let db = Database::open(&db_path)?;

            // Initialize ALLOWED_ROOTS
            ALLOWED_ROOTS
                .set(RwLock::new(vec![thumb_dir.clone()]))
                .map_err(|_| "ALLOWED_ROOTS already initialized")?;

            // Legacy: migrate old scan_dir setting
            if let Ok(Some(scan_dir)) = db.get_setting("scan_dir") {
                let scan_path = PathBuf::from(&scan_dir);
                if scan_path.exists() {
                    let _ = db.add_scan_directory(&scan_dir);
                    add_allowed_root(scan_path);
                }
            }

            // Populate allowed roots from all scan directories
            if let Ok(dirs) = db.list_scan_directories() {
                for dir_entry in dirs {
                    let scan_path = PathBuf::from(&dir_entry.path);
                    if scan_path.exists() {
                        add_allowed_root(scan_path);
                    }
                }
            }

            // Set DATA_DIR last — URI handler checks this
            DATA_DIR
                .set(app_data.clone())
                .map_err(|_| "DATA_DIR already initialized")?;

            app.manage(Arc::new(db));
            app.manage(AppDataDir(app_data));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::select_and_scan_directory,
            commands::rescan_all_directories,
            commands::rescan_directory_by_id,
            commands::load_photos,
            commands::get_photo_count,
            commands::load_photos_filtered,
            commands::photo_count_filtered,
            commands::toggle_favorite,
            commands::create_tag,
            commands::delete_tag,
            commands::rename_tag,
            commands::update_tag_color,
            commands::list_tags,
            commands::add_photo_tag,
            commands::remove_photo_tag,
            commands::get_photo_tags,
            commands::load_timeline_groups,
            commands::add_scan_directory,
            commands::remove_scan_directory,
            commands::list_scan_directories,
            commands::pick_and_add_directory,
            commands::get_settings,
            commands::update_settings,
            commands::clear_thumbnail_cache,
            commands::get_app_info,
            commands::repair_thumbnails,
        ])
        .run(tauri::generate_context!())
        .expect("Error running PhotoFlow");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_path_variants() {
        assert_eq!(uri_path("http://localhost/thumb/abc"), "/thumb/abc");
        assert_eq!(uri_path("photoflow://localhost/thumb/abc"), "/thumb/abc");
        assert_eq!(uri_path("photoflow:/thumb/abc"), "/thumb/abc");
        assert_eq!(uri_path("http://localhost/"), "/");
    }

    #[test]
    fn test_is_allowed_photo_path() {
        let root = std::path::PathBuf::from("C:\\Users\\test\\photos");
        std::fs::create_dir_all(&root).ok();
        assert!(!is_allowed_photo_path("photo.jpg", &[root.clone()]));
        assert!(!is_allowed_photo_path(
            "C:\\Users\\test\\photos\\..\\..\\secret.txt",
            &[root]
        ));
    }
}
