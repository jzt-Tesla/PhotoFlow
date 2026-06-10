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
/// Populated during setup (from DB scan_dir) and updated when user selects a new directory.
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

/// Add a directory to the allowed photo roots (called when user selects a scan directory).
pub fn add_allowed_root(path: PathBuf) {
    if let Some(lock) = ALLOWED_ROOTS.get() {
        if let Ok(mut roots) = lock.write() {
            if !roots.iter().any(|r| r == &path) {
                roots.push(path);
            }
        }
    }
}

/// Get a snapshot of the current allowed roots.
fn get_allowed_roots() -> Vec<PathBuf> {
    ALLOWED_ROOTS
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|roots| roots.clone())
        .unwrap_or_default()
}

/// Extract the path portion from a URI string.
/// Handles `http://localhost/path` (dev), `scheme://host/path` (prod), and `scheme:/path` formats.
fn uri_path(uri: &str) -> &str {
    // http://localhost/path or https://host/path
    if let Some(rest) = uri
        .strip_prefix("http://")
        .or_else(|| uri.strip_prefix("https://"))
    {
        if let Some(slash_pos) = rest.find('/') {
            return &rest[slash_pos..];
        }
        return "/";
    }
    // scheme://host/path or scheme:/path
    if let Some(idx) = uri.find(":/") {
        let after_scheme = &uri[idx + 2..]; // after ":/"
        if after_scheme.starts_with('/') {
            // scheme://host/path — skip host segment
            let host_and_path = &after_scheme[1..]; // after second /
            if let Some(host_end) = host_and_path.find('/') {
                return &host_and_path[host_end..];
            }
            return "/";
        }
        // scheme:/path — single slash, after_scheme IS the path
        return &uri[idx + 1..]; // include the "/"
    }
    uri
}

/// Validate that a decoded photo path is within one of the allowed root directories.
fn is_allowed_photo_path(file_path: &str, allowed_roots: &[PathBuf]) -> bool {
    let path = std::path::Path::new(file_path);
    // Reject relative traversal attempts
    if file_path.contains("..") {
        return false;
    }
    // Must be absolute
    if !path.is_absolute() {
        return false;
    }
    // Must be under one of the allowed roots (canonicalized to prevent bypasses)
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
        .plugin(tauri_plugin_fs::init())
        // Register the custom URI scheme on the builder.
        // The closure reads DATA_DIR and ALLOWED_ROOTS at runtime, set during setup.
        .register_uri_scheme_protocol("photoflow", |_app, request| {
            let thumb_dir = data_dir().join("thumbs");
            let allowed_roots = get_allowed_roots();

            let uri_str = request.uri().to_string();
            let path = uri_path(&uri_str);

            // Route: /thumb/{hash} → serve thumbnail JPEG from AppData/thumbs
            if let Some(hash) = path.strip_prefix("/thumb/") {
                // Validate hash is hex-only (prevents path traversal)
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

                        // SECURITY: Validate path is under an allowed directory
                        if !is_allowed_photo_path(&file_path, &allowed_roots) {
                            return http::Response::builder()
                                .status(403)
                                .body(Vec::new())
                                .unwrap();
                        }

                        match std::fs::read(&file_path) {
                            Ok(data) => {
                                // Validate magic bytes — only serve actual images
                                let mime = match detect_image_mime(&data) {
                                    Some(m) => m,
                                    None => {
                                        return http::Response::builder()
                                            .status(415) // Unsupported Media Type
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
                .expect("Failed to resolve app data dir");
            std::fs::create_dir_all(&app_data)
                .expect("Cannot create app data directory — check permissions");

            let thumb_dir = app_data.join("thumbs");
            std::fs::create_dir_all(&thumb_dir)
                .expect("Cannot create thumbnail directory — check permissions");

            let db_path = app_data.join("photoflow.db");
            let db = Database::open(&db_path).expect("Failed to open database");

            // Initialize ALLOWED_ROOTS before DATA_DIR (URI handler needs both)
            ALLOWED_ROOTS
                .set(RwLock::new(vec![thumb_dir.clone()]))
                .expect("ALLOWED_ROOTS already initialized");

            // Populate allowed roots from existing scan_dir in DB
            if let Ok(Some(scan_dir)) = db.get_setting("scan_dir") {
                let scan_path = PathBuf::from(&scan_dir);
                if scan_path.exists() {
                    if let Ok(lock) = ALLOWED_ROOTS.get().unwrap().write() {
                        // thumb_dir is already there; add the scan directory
                        drop(lock); // release write lock first
                        add_allowed_root(scan_path);
                    }
                }
            }

            // Store globally so the URI scheme handler can access it
            DATA_DIR
                .set(app_data.clone())
                .expect("DATA_DIR already initialized");

            app.manage(Arc::new(db));
            app.manage(AppDataDir(app_data));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::select_and_scan_directory,
            commands::load_photos,
            commands::get_photo_count,
            commands::rescan_directory,
        ])
        .run(tauri::generate_context!())
        .expect("Error running PhotoFlow");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_path_http() {
        assert_eq!(uri_path("http://localhost/thumb/abc123"), "/thumb/abc123");
        assert_eq!(uri_path("https://host/photo/xyz"), "/photo/xyz");
        assert_eq!(uri_path("http://localhost/"), "/");
    }

    #[test]
    fn test_uri_path_custom_scheme_double_slash() {
        assert_eq!(
            uri_path("photoflow://localhost/thumb/abc"),
            "/thumb/abc"
        );
        assert_eq!(uri_path("photoflow://localhost/"), "/");
    }

    #[test]
    fn test_uri_path_custom_scheme_single_slash() {
        assert_eq!(uri_path("photoflow:/thumb/abc"), "/thumb/abc");
        assert_eq!(uri_path("photoflow:/photo/xyz"), "/photo/xyz");
    }

    #[test]
    fn test_is_allowed_photo_path() {
        let root = std::path::PathBuf::from("C:\\Users\\test\\photos");
        std::fs::create_dir_all(&root).ok();

        assert!(!is_allowed_photo_path("photo.jpg", &[root.clone()]));
        assert!(!is_allowed_photo_path(
            "C:\\Users\\test\\photos\\..\\..\\secret.txt",
            &[root.clone()]
        ));
        assert!(!is_allowed_photo_path(
            "C:\\Users\\test\\..\\secret.txt",
            &[root]
        ));
    }
}
