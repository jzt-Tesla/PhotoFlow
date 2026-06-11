use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

// ────────────────────────── Data Structures ──────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct Photo {
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
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PhotoMeta {
    pub file_size: i64,
    pub modified_time: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanDirectory {
    pub id: i64,
    pub path: String,
    pub added_time: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TimelineGroup {
    pub year_month: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoFilter {
    #[serde(default)]
    pub favorite_only: bool,
    pub tag_id: Option<i64>,
    pub directory_path: Option<String>,
    pub year_month: Option<String>,
    pub search_query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub thumbnail_size: i32,
    pub thumbnail_quality: i32,
}

pub struct BatchCheckResult {
    /// Paths that are new (not in DB)
    pub new_paths: Vec<String>,
    /// Paths whose file_size or modified_time changed
    pub modified_paths: Vec<String>,
    /// Paths that exist in DB and are unchanged
    pub unchanged_paths: Vec<String>,
}

// ────────────────────────── Helpers ──────────────────────────

/// Escape LIKE metacharacters in user input.
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Safely remove a thumbnail file only if it resides within the expected thumbs directory.
fn safe_remove_thumbnail(thumb: &str, thumb_dir: &Path) {
    let p = Path::new(thumb);
    if let Ok(canon) = p.canonicalize() {
        if let Ok(thumb_canon) = thumb_dir.canonicalize() {
            if canon.starts_with(&thumb_canon) {
                std::fs::remove_file(canon).ok();
            }
        }
    }
}

// ────────────────────────── Database ──────────────────────────

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            ",
        )?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.create_tables()?;
        db.migrate_schema()?;

        Ok(db)
    }

    fn with_conn<F, R>(&self, f: F) -> SqlResult<R>
    where
        F: FnOnce(&Connection) -> SqlResult<R>,
    {
        let conn = self.conn.lock().map_err(|e| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                Some(format!("Database mutex poisoned (previous panic): {e}")),
            )
        })?;
        f(&conn)
    }

    // ────────────── Schema ──────────────

    fn create_tables(&self) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS photo (
                    id              INTEGER PRIMARY KEY AUTOINCREMENT,
                    path            TEXT    NOT NULL UNIQUE,
                    filename        TEXT    NOT NULL,
                    created_time    TEXT    NOT NULL,
                    width           INTEGER NOT NULL,
                    height          INTEGER NOT NULL,
                    thumbnail_path  TEXT    NOT NULL,
                    file_size       INTEGER NOT NULL DEFAULT 0,
                    modified_time   TEXT    NOT NULL DEFAULT '',
                    taken_time      TEXT,
                    camera_model    TEXT,
                    lens_model      TEXT,
                    latitude        REAL,
                    longitude       REAL,
                    favorite        INTEGER NOT NULL DEFAULT 0
                );

                CREATE TABLE IF NOT EXISTS tag (
                    id    INTEGER PRIMARY KEY AUTOINCREMENT,
                    name  TEXT NOT NULL UNIQUE,
                    color TEXT NOT NULL DEFAULT '#667eea'
                );

                CREATE TABLE IF NOT EXISTS photo_tag (
                    photo_id INTEGER NOT NULL REFERENCES photo(id) ON DELETE CASCADE,
                    tag_id   INTEGER NOT NULL REFERENCES tag(id)   ON DELETE CASCADE,
                    PRIMARY KEY (photo_id, tag_id)
                );

                CREATE TABLE IF NOT EXISTS scan_directory (
                    id         INTEGER PRIMARY KEY AUTOINCREMENT,
                    path       TEXT NOT NULL UNIQUE,
                    added_time TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                ",
            )
        })
    }

    fn migrate_schema(&self) -> SqlResult<()> {
        self.with_conn(|conn| {
            let existing: HashSet<String> = {
                let mut stmt = conn.prepare("PRAGMA table_info(photo)")?;
                let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
                rows.filter_map(|r| r.ok()).collect()
            };

            let migrations = [
                ("file_size", "INTEGER NOT NULL DEFAULT 0"),
                ("modified_time", "TEXT"),
                ("taken_time", "TEXT"),
                ("camera_model", "TEXT"),
                ("lens_model", "TEXT"),
                ("latitude", "REAL"),
                ("longitude", "REAL"),
                ("favorite", "INTEGER NOT NULL DEFAULT 0"),
            ];

            for (col, col_def) in &migrations {
                if !existing.contains(*col) {
                    conn.execute(&format!("ALTER TABLE photo ADD COLUMN {} {}", col, col_def), [])?;
                }
            }

            // Create indexes AFTER migration so all columns exist
            conn.execute_batch(
                "
                CREATE INDEX IF NOT EXISTS idx_photo_created   ON photo(created_time DESC);
                CREATE INDEX IF NOT EXISTS idx_photo_taken     ON photo(taken_time DESC);
                CREATE INDEX IF NOT EXISTS idx_photo_favorite  ON photo(favorite) WHERE favorite = 1;
                CREATE INDEX IF NOT EXISTS idx_photo_filename  ON photo(filename);
                CREATE INDEX IF NOT EXISTS idx_photo_modified  ON photo(modified_time);
                CREATE INDEX IF NOT EXISTS idx_photo_path      ON photo(path);
                CREATE INDEX IF NOT EXISTS idx_pt_photo        ON photo_tag(photo_id);
                CREATE INDEX IF NOT EXISTS idx_pt_tag          ON photo_tag(tag_id);
                ",
            )?;

            Ok(())
        })
    }

    // ────────────── Photo CRUD ──────────────

    fn photo_from_row(row: &rusqlite::Row) -> rusqlite::Result<Photo> {
        Ok(Photo {
            id: row.get(0)?,
            path: row.get(1)?,
            filename: row.get(2)?,
            created_time: row.get(3)?,
            width: row.get(4)?,
            height: row.get(5)?,
            thumbnail_path: row.get(6)?,
            file_size: row.get(7)?,
            modified_time: row.get(8)?,
            taken_time: row.get(9)?,
            camera_model: row.get(10)?,
            lens_model: row.get(11)?,
            latitude: row.get(12)?,
            longitude: row.get(13)?,
            favorite: row.get::<_, i32>(14)? != 0,
            tags: Vec::new(),
        })
    }

    const PHOTO_SELECT: &str =
        "SELECT id, path, filename, created_time, width, height, thumbnail_path,
                file_size, COALESCE(modified_time, ''), taken_time, camera_model, lens_model,
                latitude, longitude, favorite
         FROM photo";

    fn fill_tags(conn: &Connection, photos: &mut [Photo]) -> SqlResult<()> {
        if photos.is_empty() {
            return Ok(());
        }
        let ids: Vec<i64> = photos.iter().map(|p| p.id).collect();
        let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT pt.photo_id, t.name
             FROM photo_tag pt JOIN tag t ON pt.tag_id = t.id
             WHERE pt.photo_id IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut tag_map: HashMap<i64, Vec<String>> = HashMap::new();
        let rows = stmt.query_map(rusqlite::params_from_iter(ids.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for r in rows.flatten() {
            tag_map.entry(r.0).or_default().push(r.1);
        }
        for photo in photos.iter_mut() {
            photo.tags = tag_map.remove(&photo.id).unwrap_or_default();
        }
        Ok(())
    }

    /// Batch upsert with a single transaction — much faster than individual upserts.
    /// Each entry: (path, filename, created_time, width, height, thumbnail_path,
    ///              file_size, modified_time, taken_time, camera_model, lens_model,
    ///              latitude, longitude)
    pub fn upsert_photos_batch(
        &self,
        photos: &[(
            String, String, String, i32, i32, String, i64, String,
            Option<String>, Option<String>, Option<String>, Option<f64>, Option<f64>,
        )],
    ) -> SqlResult<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?; // unchecked needed: &Connection from Mutex guard
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO photo (path, filename, created_time, width, height, thumbnail_path,
                                        file_size, modified_time, taken_time, camera_model, lens_model,
                                        latitude, longitude)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
                     ON CONFLICT(path) DO UPDATE SET
                        filename=excluded.filename, created_time=excluded.created_time,
                        width=excluded.width, height=excluded.height, thumbnail_path=excluded.thumbnail_path,
                        file_size=excluded.file_size, modified_time=excluded.modified_time,
                        taken_time=excluded.taken_time, camera_model=excluded.camera_model,
                        lens_model=excluded.lens_model, latitude=excluded.latitude, longitude=excluded.longitude",
                )?;
                for (path, filename, created_time, w, h, thumb, size, mtime,
                     taken, camera, lens, lat, lon) in photos
                {
                    stmt.execute(params![
                        path, filename, created_time, *w, *h, thumb,
                        *size, mtime, taken, camera, lens, *lat, *lon
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn load_photos(&self, offset: i64, limit: i64) -> SqlResult<Vec<Photo>> {
        let offset = offset.max(0);
        let limit = limit.clamp(1, 500);
        self.with_conn(|conn| {
            let sql = format!(
                "{} ORDER BY COALESCE(taken_time, created_time) DESC LIMIT ?1 OFFSET ?2",
                Self::PHOTO_SELECT
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut photos: Vec<Photo> = stmt
                .query_map(params![limit, offset], Self::photo_from_row)?
                .collect::<SqlResult<Vec<_>>>()?;
            Self::fill_tags(conn, &mut photos)?;
            Ok(photos)
        })
    }

    pub fn load_photos_filtered(
        &self,
        filter: &PhotoFilter,
        offset: i64,
        limit: i64,
    ) -> SqlResult<Vec<Photo>> {
        let offset = offset.max(0);
        let limit = limit.clamp(1, 500);
        self.with_conn(|conn| {
            let mut wheres = Vec::new();
            let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            let mut idx = 1usize;

            if filter.favorite_only {
                wheres.push(format!("p.favorite = ?{}", idx));
                param_values.push(Box::new(1i32));
                idx += 1;
            }
            if let Some(tag_id) = filter.tag_id {
                wheres.push(format!(
                    "p.id IN (SELECT photo_id FROM photo_tag WHERE tag_id = ?{})",
                    idx
                ));
                param_values.push(Box::new(tag_id));
                idx += 1;
            }
            if let Some(ref dir_path) = filter.directory_path {
                wheres.push(format!("p.path LIKE ?{} || '%'", idx));
                param_values.push(Box::new(dir_path.clone()));
                idx += 1;
            }
            if let Some(ref ym) = filter.year_month {
                wheres.push(format!(
                    "(p.taken_time LIKE ?{} || '%' OR (p.taken_time IS NULL AND p.created_time LIKE ?{} || '%'))",
                    idx, idx
                ));
                param_values.push(Box::new(ym.clone()));
                idx += 1;
            }
            if let Some(ref q) = filter.search_query {
                if !q.is_empty() {
                    let escaped = escape_like(q);
                    wheres.push(format!(
                        "(p.filename LIKE '%' || ?{} || '%' ESCAPE '\\'
                         OR p.path LIKE '%' || ?{} || '%' ESCAPE '\\'
                         OR p.id IN (SELECT pt.photo_id FROM photo_tag pt
                                     JOIN tag t ON pt.tag_id = t.id
                                     WHERE t.name LIKE '%' || ?{} || '%' ESCAPE '\\'))",
                        idx, idx, idx
                    ));
                    param_values.push(Box::new(escaped));
                    idx += 1;
                }
            }

            let where_clause = if wheres.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", wheres.join(" AND "))
            };

            let sql = format!(
                "SELECT p.id, p.path, p.filename, p.created_time, p.width, p.height, p.thumbnail_path,
                        p.file_size, p.modified_time, p.taken_time, p.camera_model, p.lens_model,
                        p.latitude, p.longitude, p.favorite
                 FROM photo p {} ORDER BY COALESCE(p.taken_time, p.created_time) DESC LIMIT ?{} OFFSET ?{}",
                where_clause, idx, idx + 1
            );
            param_values.push(Box::new(limit));
            param_values.push(Box::new(offset));

            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();
            let mut photos: Vec<Photo> = stmt
                .query_map(param_refs.as_slice(), Self::photo_from_row)?
                .collect::<SqlResult<Vec<_>>>()?;
            Self::fill_tags(conn, &mut photos)?;
            Ok(photos)
        })
    }

    pub fn photo_count(&self) -> SqlResult<i64> {
        self.with_conn(|conn| conn.query_row("SELECT COUNT(*) FROM photo", [], |row| row.get(0)))
    }

    pub fn photo_count_filtered(&self, filter: &PhotoFilter) -> SqlResult<i64> {
        self.with_conn(|conn| {
            let mut wheres = Vec::new();
            let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            let mut idx = 1usize;

            if filter.favorite_only {
                wheres.push(format!("favorite = ?{}", idx));
                param_values.push(Box::new(1i32));
                idx += 1;
            }
            if let Some(tag_id) = filter.tag_id {
                wheres.push(format!(
                    "id IN (SELECT photo_id FROM photo_tag WHERE tag_id = ?{})",
                    idx
                ));
                param_values.push(Box::new(tag_id));
                idx += 1;
            }
            if let Some(ref dir_path) = filter.directory_path {
                wheres.push(format!("path LIKE ?{} || '%'", idx));
                param_values.push(Box::new(dir_path.clone()));
                idx += 1;
            }
            if let Some(ref ym) = filter.year_month {
                wheres.push(format!(
                    "(taken_time LIKE ?{} || '%' OR (taken_time IS NULL AND created_time LIKE ?{} || '%'))",
                    idx, idx
                ));
                param_values.push(Box::new(ym.clone()));
                idx += 1;
            }
            if let Some(ref q) = filter.search_query {
                if !q.is_empty() {
                    let escaped = escape_like(q);
                    wheres.push(format!(
                        "(filename LIKE '%' || ?{} || '%' ESCAPE '\\'
                         OR path LIKE '%' || ?{} || '%' ESCAPE '\\'
                         OR id IN (SELECT pt.photo_id FROM photo_tag pt
                                   JOIN tag t ON pt.tag_id = t.id
                                   WHERE t.name LIKE '%' || ?{} || '%' ESCAPE '\\'))",
                        idx, idx, idx
                    ));
                    param_values.push(Box::new(escaped));
                }
            }

            let where_clause = if wheres.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", wheres.join(" AND "))
            };
            let sql = format!("SELECT COUNT(*) FROM photo {}", where_clause);
            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();
            stmt.query_row(param_refs.as_slice(), |row| row.get(0))
        })
    }

    // ────────────── Incremental scan support ──────────────

    pub fn get_photo_meta(&self, path: &str) -> SqlResult<Option<PhotoMeta>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT file_size, COALESCE(modified_time, '') FROM photo WHERE path = ?1",
            )?;
            let mut rows = stmt.query_map(params![path], |row| {
                Ok(PhotoMeta {
                    file_size: row.get(0)?,
                    modified_time: row.get(1)?,
                })
            })?;
            match rows.next() {
                Some(Ok(meta)) => Ok(Some(meta)),
                _ => Ok(None),
            }
        })
    }

    /// Returns paths not in the DB
    pub fn get_new_paths(&self, paths: &[String]) -> SqlResult<Vec<String>> {
        self.with_conn(|conn| {
            let mut existing = HashSet::new();
            {
                let mut stmt = conn.prepare("SELECT path FROM photo")?;
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
                for r in rows.flatten() {
                    existing.insert(r);
                }
            }
            Ok(paths
                .iter()
                .filter(|p| !existing.contains(*p))
                .cloned()
                .collect())
        })
    }

    /// Batch check: given (path, file_size, modified_time), return which are new/modified/unchanged
    pub fn batch_check_existing(
        &self,
        files: &[(String, i64, String)],
    ) -> SqlResult<BatchCheckResult> {
        self.with_conn(|conn| {
            let mut existing: HashMap<String, (i64, String)> = HashMap::new();
            {
                let mut stmt =
                    conn.prepare("SELECT path, file_size, COALESCE(modified_time, '') FROM photo")?;
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        (row.get::<_, i64>(1)?, row.get::<_, String>(2)?),
                    ))
                })?;
                for r in rows.flatten() {
                    existing.insert(r.0, r.1);
                }
            }

            let mut result = BatchCheckResult {
                new_paths: Vec::new(),
                modified_paths: Vec::new(),
                unchanged_paths: Vec::new(),
            };

            for (path, size, mtime) in files {
                match existing.get(path) {
                    None => result.new_paths.push(path.clone()),
                    Some((db_size, db_mtime)) => {
                        if *db_size != *size || *db_mtime != *mtime {
                            result.modified_paths.push(path.clone());
                        } else {
                            result.unchanged_paths.push(path.clone());
                        }
                    }
                }
            }
            Ok(result)
        })
    }

    /// Delete photos whose paths are NOT in the given set
    pub fn delete_missing_photos(&self, existing_paths: &HashSet<String>, thumb_dir: &Path) -> SqlResult<usize> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT id, path, thumbnail_path FROM photo")?;
            let orphans: Vec<(i64, String, String)> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .filter_map(|r| r.ok())
                .filter(|(_, path, _)| !existing_paths.contains(path))
                .collect();

            let count = orphans.len();
            for (id, _, thumb) in &orphans {
                conn.execute("DELETE FROM photo WHERE id = ?1", params![id])?;
                safe_remove_thumbnail(thumb, thumb_dir);
            }
            Ok(count)
        })
    }

    pub fn delete_photo_by_path(&self, path: &str) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM photo WHERE path = ?1", params![path])?;
            Ok(())
        })
    }

    pub fn cleanup_orphans(&self, thumb_dir: &Path) -> usize {
        let orphans: Vec<(i64, String, String)> = match self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT id, path, thumbnail_path FROM photo")?;
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

        let mut orphan_ids: Vec<i64> = Vec::new();
        for (id, photo_path, thumb_path) in &orphans {
            if !Path::new(photo_path).exists() {
                safe_remove_thumbnail(thumb_path, thumb_dir);
                orphan_ids.push(*id);
            }
        }

        if !orphan_ids.is_empty() {
            let _ = self.with_conn(|conn| {
                for id in &orphan_ids {
                    conn.execute("DELETE FROM photo WHERE id = ?1", params![id])?;
                }
                Ok(())
            });
        }

        if thumb_dir.exists() {
            let db_thumbs: HashSet<String> = self
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

    // ────────────── Favorites ──────────────

    pub fn toggle_favorite(&self, photo_id: i64) -> SqlResult<bool> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE photo SET favorite = CASE WHEN favorite = 1 THEN 0 ELSE 1 END WHERE id = ?1",
                params![photo_id],
            )?;
            let fav: i32 =
                conn.query_row("SELECT favorite FROM photo WHERE id = ?1", params![photo_id], |r| r.get(0))?;
            Ok(fav != 0)
        })
    }

    // ────────────── Tags ──────────────

    pub fn create_tag(&self, name: &str, color: &str) -> SqlResult<i64> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO tag (name, color) VALUES (?1, ?2)",
                params![name, color],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    pub fn delete_tag(&self, tag_id: i64) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM tag WHERE id = ?1", params![tag_id])?;
            Ok(())
        })
    }

    pub fn rename_tag(&self, tag_id: i64, new_name: &str) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE tag SET name = ?1 WHERE id = ?2",
                params![new_name, tag_id],
            )?;
            Ok(())
        })
    }

    pub fn update_tag_color(&self, tag_id: i64, color: &str) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE tag SET color = ?1 WHERE id = ?2",
                params![color, tag_id],
            )?;
            Ok(())
        })
    }

    pub fn list_tags(&self) -> SqlResult<Vec<Tag>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.name, t.color, COUNT(pt.photo_id) as cnt
                 FROM tag t LEFT JOIN photo_tag pt ON t.id = pt.tag_id
                 GROUP BY t.id ORDER BY t.name",
            )?;
            let tags = stmt
                .query_map([], |row| {
                    Ok(Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        color: row.get(2)?,
                        count: row.get(3)?,
                    })
                })?
                .collect::<SqlResult<Vec<_>>>()?;
            Ok(tags)
        })
    }

    pub fn add_photo_tag(&self, photo_id: i64, tag_id: i64) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO photo_tag (photo_id, tag_id) VALUES (?1, ?2)",
                params![photo_id, tag_id],
            )?;
            Ok(())
        })
    }

    pub fn remove_photo_tag(&self, photo_id: i64, tag_id: i64) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM photo_tag WHERE photo_id = ?1 AND tag_id = ?2",
                params![photo_id, tag_id],
            )?;
            Ok(())
        })
    }

    pub fn get_photo_tags(&self, photo_id: i64) -> SqlResult<Vec<Tag>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.name, t.color,
                        (SELECT COUNT(*) FROM photo_tag WHERE tag_id = t.id) as cnt
                 FROM tag t
                 JOIN photo_tag pt ON t.id = pt.tag_id
                 WHERE pt.photo_id = ?1 ORDER BY t.name",
            )?;
            stmt.query_map(params![photo_id], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    count: row.get(3)?,
                })
            })
            .and_then(|rows| rows.collect())
        })
    }

    // ────────────── Timeline ──────────────

    pub fn load_timeline_groups(&self) -> SqlResult<Vec<TimelineGroup>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT substr(COALESCE(taken_time, created_time), 1, 7) as ym, COUNT(*)
                 FROM photo
                 WHERE COALESCE(taken_time, created_time) IS NOT NULL
                   AND COALESCE(taken_time, created_time) != ''
                 GROUP BY ym ORDER BY ym DESC",
            )?;
            stmt.query_map([], |row| {
                Ok(TimelineGroup {
                    year_month: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .and_then(|rows| rows.collect())
        })
    }

    // ────────────── Multi-directory ──────────────

    pub fn add_scan_directory(&self, path: &str) -> SqlResult<i64> {
        self.with_conn(|conn| {
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            conn.execute(
                "INSERT OR IGNORE INTO scan_directory (path, added_time) VALUES (?1, ?2)",
                params![path, now],
            )?;
            conn.query_row(
                "SELECT id FROM scan_directory WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
        })
    }

    pub fn remove_scan_directory(&self, dir_id: i64) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM scan_directory WHERE id = ?1", params![dir_id])?;
            Ok(())
        })
    }

    pub fn list_scan_directories(&self) -> SqlResult<Vec<ScanDirectory>> {
        self.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT id, path, added_time FROM scan_directory ORDER BY added_time DESC")?;
            stmt.query_map([], |row| {
                Ok(ScanDirectory {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    added_time: row.get(2)?,
                })
            })
            .and_then(|rows| rows.collect())
        })
    }

    // ────────────── Settings ──────────────

    pub fn set_setting(&self, key: &str, value: &str) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
            Ok(())
        })
    }

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

    pub fn get_app_settings(&self) -> AppSettings {
        let thumbnail_size = self
            .get_setting("thumbnail_size")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(200);
        let thumbnail_quality = self
            .get_setting("thumbnail_quality")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(80);
        AppSettings {
            thumbnail_size,
            thumbnail_quality,
        }
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> SqlResult<()> {
        self.set_setting("thumbnail_size", &settings.thumbnail_size.to_string())?;
        self.set_setting("thumbnail_quality", &settings.thumbnail_quality.to_string())?;
        Ok(())
    }

    /// Get all photo paths and their thumbnail paths (for repair).
    pub fn get_all_photo_paths_and_thumbs(&self) -> SqlResult<Vec<(String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT path, thumbnail_path FROM photo")?;
            let rows = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<SqlResult<Vec<_>>>()?;
            Ok(rows)
        })
    }

    /// Update the thumbnail path for a photo.
    pub fn update_thumbnail_path(&self, photo_path: &str, new_thumb_path: &str) -> SqlResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE photo SET thumbnail_path = ?1 WHERE path = ?2",
                params![new_thumb_path, photo_path],
            )?;
            Ok(())
        })
    }
}
