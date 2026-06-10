use rusqlite::{params, Connection, Result as SqlResult};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

/// A photo record stored in the database.
#[derive(Debug, Serialize, Clone)]
pub struct Photo {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub created_time: String,
    pub width: i32,
    pub height: i32,
    pub thumbnail_path: String,
}

/// Thread-safe wrapper around a SQLite connection.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the database at `path` and ensure the schema exists.
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS photo (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                path            TEXT    NOT NULL UNIQUE,
                filename        TEXT    NOT NULL,
                created_time    TEXT    NOT NULL,
                width           INTEGER NOT NULL,
                height          INTEGER NOT NULL,
                thumbnail_path  TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_photo_created ON photo(created_time DESC);
            CREATE TABLE IF NOT EXISTS settings (
                key     TEXT PRIMARY KEY,
                value   TEXT NOT NULL
            );
            ",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Execute a closure with access to the database connection.
    /// Recovers from mutex poisoning instead of panicking.
    fn with_conn<F, R>(&self, f: F) -> SqlResult<R>
    where
        F: FnOnce(&Connection) -> SqlResult<R>,
    {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        f(&conn)
    }

    /// Insert a photo record, skipping duplicates (same path).
    pub fn insert_photo(
        &self,
        path: &str,
        filename: &str,
        created_time: &str,
        width: i32,
        height: i32,
        thumbnail_path: &str,
    ) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO photo (path, filename, created_time, width, height, thumbnail_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![path, filename, created_time, width, height, thumbnail_path],
            )?;
            Ok(())
        })
    }

    /// Insert a batch of photo records in a single transaction.
    /// Each entry is (path, filename, created_time, width, height, thumbnail_path).
    pub fn insert_photos_batch(
        &self,
        photos: &[(String, String, String, i32, i32, String)],
    ) -> SqlResult<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR IGNORE INTO photo (path, filename, created_time, width, height, thumbnail_path)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )?;
                for (path, filename, created_time, width, height, thumbnail_path) in photos {
                    stmt.execute(params![
                        path, filename, created_time, width, height, thumbnail_path
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// Return photos ordered by created_time DESC, with offset/limit pagination.
    /// Offset and limit are clamped to safe ranges.
    pub fn load_photos(&self, offset: i64, limit: i64) -> SqlResult<Vec<Photo>> {
        let offset = offset.max(0);
        let limit = limit.clamp(1, 500);
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, filename, created_time, width, height, thumbnail_path
                 FROM photo
                 ORDER BY created_time DESC
                 LIMIT ?1 OFFSET ?2",
            )?;
            let rows = stmt.query_map(params![limit, offset], |row| {
                Ok(Photo {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    filename: row.get(2)?,
                    created_time: row.get(3)?,
                    width: row.get(4)?,
                    height: row.get(5)?,
                    thumbnail_path: row.get(6)?,
                })
            })?;
            rows.collect()
        })
    }

    /// Return the total number of photos in the database.
    pub fn photo_count(&self) -> SqlResult<i64> {
        self.with_conn(|conn| conn.query_row("SELECT COUNT(*) FROM photo", [], |row| row.get(0)))
    }

    /// Remove DB records whose source files no longer exist,
    /// and delete their orphaned thumbnail files.
    /// Returns the number of removed records.
    /// Uses phased locking: collect → delete files → delete records.
    pub fn cleanup_orphans(&self, thumb_dir: &Path) -> usize {
        // Phase 1: collect orphans while holding lock briefly
        let orphans: Vec<(i64, String, String)> = match self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id, path, thumbnail_path FROM photo")
                .map_err(|e| e)?;
            let rows: Vec<(i64, String, String)> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }) {
            Ok(rows) => rows,
            Err(_) => return 0,
        };

        // Phase 2: delete orphan files (no lock held)
        let mut orphan_ids: Vec<i64> = Vec::new();
        for (id, photo_path, thumb_path) in &orphans {
            if !Path::new(photo_path).exists() {
                std::fs::remove_file(thumb_path).ok();
                orphan_ids.push(*id);
            }
        }

        // Phase 3: delete DB records (hold lock briefly)
        if !orphan_ids.is_empty() {
            let _ = self.with_conn(|conn| {
                for id in &orphan_ids {
                    conn.execute("DELETE FROM photo WHERE id = ?1", params![id])?;
                }
                Ok(())
            });
        }

        // Phase 4: clean up orphaned thumbnail files not referenced in DB
        if thumb_dir.exists() {
            let db_thumbs: std::collections::HashSet<String> = self
                .with_conn(|conn| {
                    let mut stmt = conn.prepare("SELECT thumbnail_path FROM photo")?;
                    let rows: Vec<String> = stmt
                        .query_map([], |row| row.get::<_, String>(0))?
                        .filter_map(|r| r.ok())
                        .collect();
                    Ok(rows)
                })
                .unwrap_or_default()
                .into_iter()
                .collect();

            if let Ok(entries) = std::fs::read_dir(thumb_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    let path_str = path.to_string_lossy().to_string();
                    if !db_thumbs.contains(&path_str) {
                        std::fs::remove_file(&path).ok();
                    }
                }
            }
        }

        orphan_ids.len()
    }

    /// Store a key-value setting.
    pub fn set_setting(&self, key: &str, value: &str) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
            Ok(())
        })
    }

    /// Retrieve a setting value by key.
    pub fn get_setting(&self, key: &str) -> SqlResult<Option<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
            let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
            match rows.next() {
                Some(Ok(val)) => Ok(Some(val)),
                _ => Ok(None),
            }
        })
    }
}
