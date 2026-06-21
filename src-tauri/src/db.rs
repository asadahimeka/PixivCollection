use std::sync::{Mutex, MutexGuard, OnceLock};

use rusqlite::{params, Connection, Transaction};

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnsureDbResult {
    pub status: String,
    pub imported: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportResult {
    pub imported: i64,
    pub skipped: i64,
}

fn create_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS images (
            id INTEGER NOT NULL,
            part INTEGER NOT NULL,
            len INTEGER NOT NULL DEFAULT 1,
            title TEXT NOT NULL DEFAULT '',
            width INTEGER NOT NULL DEFAULT 0,
            height INTEGER NOT NULL DEFAULT 0,
            ext TEXT NOT NULL DEFAULT '',
            author_id INTEGER NOT NULL DEFAULT 0,
            author_name TEXT NOT NULL DEFAULT '',
            author_account TEXT NOT NULL DEFAULT '',
            bookmark INTEGER NOT NULL DEFAULT 0,
            view INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT '',
            sanity_level INTEGER NOT NULL DEFAULT 0,
            x_restrict INTEGER NOT NULL DEFAULT 0,
            is_ai INTEGER NOT NULL DEFAULT 0,
            img_s TEXT NOT NULL DEFAULT '',
            img_m TEXT NOT NULL DEFAULT '',
            img_l TEXT NOT NULL DEFAULT '',
            img_o TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (id, part)
        );
        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            translated_name TEXT
        );
        CREATE TABLE IF NOT EXISTS image_tags (
            image_id INTEGER NOT NULL,
            image_part INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (image_id, image_part, tag_id),
            FOREIGN KEY (image_id, image_part) REFERENCES images(id, part),
            FOREIGN KEY (tag_id) REFERENCES tags(id)
        );
        CREATE TABLE IF NOT EXISTS _meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .map_err(|e| format!("Failed to create schema: {e}"))
}

fn create_indexes(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_images_author ON images(author_id);
         CREATE INDEX IF NOT EXISTS idx_images_bookmark ON images(bookmark);
         CREATE INDEX IF NOT EXISTS idx_images_restrict ON images(x_restrict);
         CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at);
         CREATE INDEX IF NOT EXISTS idx_images_size ON images(width, height);
         CREATE INDEX IF NOT EXISTS idx_images_is_ai ON images(is_ai);
         CREATE INDEX IF NOT EXISTS idx_image_tags_tag ON image_tags(tag_id);
         CREATE INDEX IF NOT EXISTS idx_image_tags_image ON image_tags(image_id, image_part);
         CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);",
    )
    .map_err(|e| format!("Failed to create indexes: {e}"))
}

/// Initialize the database connection and create schema.
/// Opens or creates the SQLite file at `db_path`, enables WAL mode
/// and foreign keys, then creates all tables if they don't exist.
pub fn init_db(db_path: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open database: {e}"))?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("Failed to set pragmas: {e}"))?;

    create_schema(&conn)?;

    DB.set(Mutex::new(conn))
        .map_err(|_| "Database already initialized".to_string())?;

    Ok(())
}

/// Get a locked reference to the global database connection.
/// Returns an error if `init_db` has not been called yet.
pub fn get_conn() -> Result<MutexGuard<'static, Connection>, String> {
    DB.get()
        .ok_or_else(|| "Database not initialized".to_string())?
        .lock()
        .map_err(|e| format!("Failed to lock database mutex: {e}"))
}

/// Core insert logic shared by `ensure_db` and `import_json_to_db_logic`.
///
/// Parses a JSON array of image objects, bulk-inserts into `images`,
/// normalises tags into `tags` / `image_tags`, and returns the count of
/// rows inserted vs skipped (duplicate primary key).
fn import_in_transaction(tx: &Transaction<'_>, json_content: &str) -> Result<ImportResult, String> {
    let items: Vec<serde_json::Value> =
        serde_json::from_str(json_content).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    let mut imported = 0i64;
    let mut skipped = 0i64;

    for item in &items {
        let id = item["id"].as_i64().unwrap_or(0);
        let part = item["part"].as_i64().unwrap_or(0);
        let len = item["len"].as_i64().unwrap_or(1);
        let title = item["title"].as_str().unwrap_or("");
        let ext = item["ext"].as_str().unwrap_or("");

        let width = item["size"][0].as_i64().unwrap_or(0);
        let height = item["size"][1].as_i64().unwrap_or(0);

        let author_id = item["author"]["id"].as_i64().unwrap_or(0);
        let author_name = item["author"]["name"].as_str().unwrap_or("");
        let author_account = item["author"]["account"].as_str().unwrap_or("");

        let bookmark = item["bookmark"].as_i64().unwrap_or(0);
        let view = item["view"].as_i64().unwrap_or(0);
        let created_at = item["created_at"].as_str().unwrap_or("");
        let sanity_level = item["sanity_level"].as_i64().unwrap_or(0);
        let x_restrict = item["x_restrict"].as_i64().unwrap_or(0);
        let is_ai = if item["isAI"].as_bool().unwrap_or(false) {
            1i64
        } else {
            0i64
        };

        let img_s = item["images"]["s"].as_str().unwrap_or("");
        let img_m = item["images"]["m"].as_str().unwrap_or("");
        let img_l = item["images"]["l"].as_str().unwrap_or("");
        let img_o = item["images"]["o"].as_str().unwrap_or("");

        let rows = tx
            .execute(
                "INSERT OR IGNORE INTO images \
                 (id, part, len, title, width, height, ext, \
                  author_id, author_name, author_account, bookmark, view, created_at, \
                  sanity_level, x_restrict, is_ai, \
                  img_s, img_m, img_l, img_o) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, \
                         ?8, ?9, ?10, ?11, ?12, ?13, \
                         ?14, ?15, ?16, \
                         ?17, ?18, ?19, ?20)",
                params![
                    id, part, len, title, width, height, ext, author_id, author_name,
                    author_account, bookmark, view, created_at, sanity_level, x_restrict, is_ai,
                    img_s, img_m, img_l, img_o,
                ],
            )
            .map_err(|e| format!("Failed to insert image {id} p{part}: {e}"))?;

        if rows > 0 {
            imported += 1;
        } else {
            skipped += 1;
            // Image already exists – skip tag processing (links already present)
            continue;
        }

        if let Some(tags) = item["tags"].as_array() {
            for tag_obj in tags {
                let name = tag_obj["name"].as_str().unwrap_or("");
                let translated_name = tag_obj["translated_name"].as_str();

                tx.execute(
                    "INSERT OR IGNORE INTO tags (name, translated_name) VALUES (?1, ?2)",
                    params![name, translated_name],
                )
                .map_err(|e| format!("Failed to insert tag '{name}': {e}"))?;

                let tag_id: i64 = tx
                    .query_row("SELECT id FROM tags WHERE name = ?1", params![name], |row| {
                        row.get(0)
                    })
                    .map_err(|e| format!("Failed to get tag id for '{name}': {e}"))?;

                tx.execute(
                    "INSERT OR IGNORE INTO image_tags (image_id, image_part, tag_id) \
                     VALUES (?1, ?2, ?3)",
                    params![id, part, tag_id],
                )
                .map_err(|e| {
                    format!(
                        "Failed to insert image_tag for {id} p{part} tag '{name}': {e}"
                    )
                })?;
            }
        }
    }

    Ok(ImportResult { imported, skipped })
}

/// Ensure the SQLite database is fully populated.
///
/// If `{img_dir}/images.db` already exists and has been imported before
/// (indicated by a `schema_version` row in `_meta`), checks whether the
/// caller-supplied `version` differs from the stored `json_version`.
///   - Version matches (or no version supplied) → returns `"exists"` immediately.
///   - Version differs → re-imports from `images.json` (INSERT OR IGNORE).
///
/// Otherwise reads `{img_dir}/images.json`, bulk-imports every entry,
/// creates indexes, and records both `schema_version` and `json_version`.
pub fn ensure_db(img_dir: &str, version: Option<i64>) -> Result<EnsureDbResult, String> {
    let data_dir = format!("{img_dir}/data");
    let db_path = format!("{data_dir}/images.db");
    if !std::path::Path::new(&data_dir).exists() {
        std::fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {e}"))?;
    }
    let db_exists = std::path::Path::new(&db_path).exists();

    if DB.get().is_none() {
        init_db(&db_path)?;
    }

    if db_exists {
        let conn = get_conn()?;
        let has_schema: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM _meta WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .map_err(|e| format!("Failed to check schema version: {e}"))?;

        if has_schema {
            let needs_reimport = match version {
                Some(v) => {
                    let stored = conn
                        .query_row(
                            "SELECT value FROM _meta WHERE key = 'json_version'",
                            [],
                            |row| row.get::<_, String>(0),
                        )
                        .ok()
                        .and_then(|s| s.parse::<i64>().ok());
                    stored != Some(v)
                }
                None => false,
            };

            if !needs_reimport {
                let imported: i64 = conn
                    .query_row("SELECT COUNT(*) FROM images", [], |row| row.get(0))
                    .map_err(|e| format!("Failed to count images: {e}"))?;
                return Ok(EnsureDbResult {
                    status: "exists".to_string(),
                    imported,
                });
            }
        }
        // No schema_version marker – treat as stale / empty DB and re-import.
    }

    let json_path = format!("{data_dir}/images.json");
    // If images.json does not exist, the user has never fetched bookmarks.
    // Return "no_data" so the frontend can show the appropriate empty state
    // instead of crashing.
    if !std::path::Path::new(&json_path).exists() {
        return Ok(EnsureDbResult {
            status: "no_data".to_string(),
            imported: 0,
        });
    }
    let json_content =
        std::fs::read_to_string(&json_path).map_err(|e| format!("Failed to read images.json: {e}"))?;

    let mut conn = get_conn()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {e}"))?;

    let result = import_in_transaction(&tx, &json_content)?;

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    // Create indexes after bulk insert for performance.
    create_indexes(&conn)?;

    conn.execute(
        "INSERT OR IGNORE INTO _meta (key, value) VALUES ('schema_version', '1')",
        [],
    )
    .map_err(|e| format!("Failed to store schema version: {e}"))?;

    if let Some(v) = version {
        conn.execute(
            "INSERT OR REPLACE INTO _meta (key, value) VALUES ('json_version', ?1)",
            params![v.to_string()],
        )
        .map_err(|e| format!("Failed to store json_version: {e}"))?;
    }

    Ok(EnsureDbResult {
        status: "created".to_string(),
        imported: result.imported,
    })
}

/// Import image data from a raw JSON string.
///
/// Parses `json_content` as an array of image objects, inserts every
/// entry (with `INSERT OR IGNORE` so duplicates are skipped), and
/// ensures indexes exist.
///
/// Returns the number of rows imported vs skipped.
pub fn import_json_to_db_logic(img_dir: &str, json_content: &str) -> Result<ImportResult, String> {
    // Lazy-init if this is called before ensure_db
    if DB.get().is_none() {
        let data_dir = format!("{img_dir}/data");
        if !std::path::Path::new(&data_dir).exists() {
            std::fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {e}"))?;
        }
        let db_path = format!("{data_dir}/images.db");
        init_db(&db_path)?;
    }

    let mut conn = get_conn()?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {e}"))?;

    let result = import_in_transaction(&tx, json_content)?;

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    // Idempotent – no-op if indexes already exist.
    create_indexes(&conn)?;

    Ok(result)
}
