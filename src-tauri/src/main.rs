#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod db;
mod query;

use rusqlite::types::ToSql;
use tauri::Manager;
use warp::Filter;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[tauri::command]
fn start_local_server(base: String) -> Result<(), String> {
    let static_dir = warp::fs::dir(base);

    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    let routes = static_dir.with(cors);

    tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], 32154)).await;
    });

    Ok(())
}

#[tauri::command]
fn get_executable_dir() -> Result<std::path::PathBuf, String> {
    match std::env::current_exe() {
        Ok(path) => match path.parent() {
            Some(parent) => Ok(parent.to_path_buf()),
            None => Err("Failed to get parent directory".to_string()),
        },
        Err(error) => Err(format!("{error}")),
    }
}

#[tauri::command]
fn restart_app(app_handle: tauri::AppHandle) {
    app_handle.restart();
}

/// Ensure the SQLite database is populated from images.json.
/// Called once on app startup before any query.
/// `version` is an optional JSON version counter from localStorage.
/// When provided and different from the stored version, a re-import is triggered.
/// Emits `import-progress` events to the frontend during the initial import.
#[tauri::command]
fn ensure_db(app: tauri::AppHandle, img_dir: String, version: Option<i64>) -> Result<db::EnsureDbResult, String> {
    let emit = |current: usize, total: usize| {
        let _ = app.emit_all(
            "import-progress",
            serde_json::json!({ "current": current, "total": total }),
        );
    };
    db::ensure_db(&img_dir, version, Some(&emit))
}

/// Paginated, filtered image query against the SQLite database.
/// Supports all filter dimensions from the original JS imageFilter().
/// Returns only the image rows (no aggregate counts — use query_image_counts).
#[tauri::command]
fn query_images(query: query::ImageQuery) -> Result<query::QueryResult, String> {
    let conn = db::get_conn()?;

    let (sql, params) = query.build_sql();
    let images: Vec<query::ImageRow> = {
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {e}"))?;
        let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(&refs),
                query::image_row_from_row,
            )
            .map_err(|e| format!("Query failed: {e}"))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("Row mapping failed: {e}"))?);
        }
        result
    };

    // 2b. Multi-page boundary: if the last image is part of a multi-page
    //     illustration and not all pages are included, fetch the continuation.
    let images = if let Some(last) = images.last() {
        if last.part < last.len - 1 {
            let needed = last.len - last.part - 1;
            let extra_sql = concat!(
                "SELECT i.id, i.part, i.len, i.title, i.width, i.height, i.ext, ",
                "i.author_id, i.author_name, i.author_account, i.bookmark, i.view, ",
                "i.created_at, i.sanity_level, i.x_restrict, i.is_ai, ",
                "i.img_s, i.img_m, i.img_l, i.img_o FROM images i ",
                "WHERE i.id = ?1 AND i.part > ?2 ORDER BY i.part ASC LIMIT ?3",
            );
            let mut stmt = conn
                .prepare(extra_sql)
                .map_err(|e| format!("Prepare extra failed: {e}"))?;
            let extra_rows = stmt
                .query_map(
                    rusqlite::params![last.id, last.part, needed],
                    query::image_row_from_row,
                )
                .map_err(|e| format!("Extra query failed: {e}"))?;
            let mut all = images;
            for row in extra_rows {
                all.push(row.map_err(|e| format!("Extra row mapping failed: {e}"))?);
            }
            all
        } else {
            images
        }
    } else {
        images
    };

    // ---- Batch fetch tags ----
    let mut tag_map: std::collections::HashMap<(i64, i64), Vec<query::ImageTag>> =
        std::collections::HashMap::new();
    {
        let ids: Vec<i64> = images.iter().map(|img| img.id).collect();
        if !ids.is_empty() {
            let ph: Vec<String> = (1..=ids.len()).map(|i| format!("?{i}")).collect();
            let tag_sql = format!(
                "SELECT it.image_id, it.image_part, t.name, t.translated_name \
                 FROM image_tags it \
                 JOIN tags t ON t.id = it.tag_id \
                 WHERE it.image_id IN ({})",
                ph.join(","),
            );
            let mut tag_stmt = conn
                .prepare(&tag_sql)
                .map_err(|e| format!("Tag prepare failed: {e}"))?;
            let tag_refs: Vec<&dyn ToSql> = ids.iter().map(|id| id as &dyn ToSql).collect();
            let tag_rows = tag_stmt
                .query_map(rusqlite::params_from_iter(&tag_refs), |row| {
                    let image_id: i64 = row.get("image_id")?;
                    let image_part: i64 = row.get("image_part")?;
                    let name: String = row.get("name")?;
                    let translated_name: Option<String> = row.get("translated_name")?;
                    Ok((image_id, image_part, query::ImageTag { name, translated_name }))
                })
                .map_err(|e| format!("Tag query failed: {e}"))?;
            for tag_row in tag_rows {
                let (image_id, image_part, tag) = tag_row.map_err(|e| format!("Tag row failed: {e}"))?;
                tag_map.entry((image_id, image_part)).or_default().push(tag);
            }
        }
    }
    let images: Vec<query::ImageRow> = images
        .into_iter()
        .map(|mut img| {
            if let Some(tags) = tag_map.remove(&(img.id, img.part)) {
                img.tags = tags;
            }
            img
        })
        .collect();

    Ok(query::QueryResult { images })
}

/// Aggregate counts for the current filter (total, illust_count,
/// author_count, tag_count).
#[tauri::command]
fn query_image_counts(query: query::ImageQuery) -> Result<query::CountsResult, String> {
    let conn = db::get_conn()?;

    let (csql, cparams) = query.build_counts_sql();
    let refs: Vec<&dyn ToSql> = cparams.iter().map(|b| b.as_ref()).collect();
    let (total, illust_count, author_count): (i64, i64, i64) = conn
        .query_row(
            &csql,
            rusqlite::params_from_iter(&refs),
            |row| Ok((row.get("total")?, row.get("illust_count")?, row.get("author_count")?)),
        )
        .map_err(|e| format!("Counts query failed: {e}"))?;

    let (tcsql, tcparams) = query.build_tag_count_sql();
    let tcrefs: Vec<&dyn ToSql> = tcparams.iter().map(|b| b.as_ref()).collect();
    let tag_count: i64 = conn
        .query_row(
            &tcsql,
            rusqlite::params_from_iter(&tcrefs),
            |row| row.get(0),
        )
        .map_err(|e| format!("Tag count query failed: {e}"))?;

    Ok(query::CountsResult { total, illust_count, author_count, tag_count })
}

/// Read the cached full counts from the _meta table.
/// These are populated by refresh_caches / ensure_db and include
/// total, illustCount, authorCount, tagCount for the entire dataset.
#[tauri::command]
fn get_full_counts() -> Result<serde_json::Value, String> {
    let conn = db::get_conn()?;
    let val: String = conn
        .query_row(
            "SELECT value FROM _meta WHERE key = 'full_counts'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Full counts not cached: {e}"))?;
    serde_json::from_str(&val).map_err(|e| format!("Parse failed: {e}"))
}

/// Refresh all cached aggregate data in the _meta table.
/// Called after data import to update sidebar filters and full counts.
#[tauri::command]
fn refresh_caches(img_dir: String, version: Option<i64>) -> Result<(), String> {
    db::refresh_caches(&img_dir, version)
}

/// Re-import images.json into the SQLite database, then refresh caches.
/// Unlike import_json_to_db (which takes raw JSON as a string argument),
/// this command reads images.json directly from disk.
/// Emits `import-progress` events to the frontend during the import.
#[tauri::command]
fn reimport_db(app: tauri::AppHandle, img_dir: String, version: Option<i64>) -> Result<db::ImportResult, String> {
    let json_path = format!("{img_dir}/data/images.json");
    let json_content = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("Failed to read images.json: {e}"))?;
    let emit = |current: usize, total: usize| {
        let _ = app.emit_all(
            "import-progress",
            serde_json::json!({ "current": current, "total": total }),
        );
    };
    let result = db::import_json_to_db_logic(&img_dir, &json_content, Some(&emit))?;
    db::refresh_caches(&img_dir, version)?;
    Ok(result)
}

/// Aggregate filter options (years, authors, tags with counts) for the sidebar.
/// Reads from cached data in _meta table. Falls back to full DB scan on cache miss.
#[tauri::command]
fn get_filter_options() -> Result<query::FilterOptions, String> {
    let conn = db::get_conn()?;

    fn read_json<T: serde::de::DeserializeOwned>(
        conn: &rusqlite::Connection,
        key: &str,
        fallback: fn(&rusqlite::Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let val: String = match conn.query_row(
            "SELECT value FROM _meta WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        ) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[WARN] Cache '{key}' not found: {e}, falling back to full scan");
                return fallback(conn);
            }
        };
        match serde_json::from_str(&val) {
            Ok(v) => Ok(v),
            Err(e) => {
                eprintln!("[WARN] Cache '{key}' parse failed: {e}, falling back to full scan");
                fallback(conn)
            }
        }
    }

    fn fallback_years(conn: &rusqlite::Connection) -> Result<Vec<query::YearOption>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT CAST(substr(created_at,1,4) AS INTEGER) as year, \
                 COUNT(*) as count FROM images \
                 GROUP BY year ORDER BY year DESC",
            )
            .map_err(|e| format!("Years fallback failed: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(query::YearOption {
                    year: row.get("year")?,
                    count: row.get("count")?,
                })
            })
            .map_err(|e| format!("Years query failed: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Years collect failed: {e}"))
    }

    fn fallback_authors(conn: &rusqlite::Connection) -> Result<Vec<query::AuthorOption>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT author_id as id, author_name as name, \
                 author_account as account, COUNT(DISTINCT id) as count \
                 FROM images GROUP BY author_id \
                 ORDER BY count DESC LIMIT 100",
            )
            .map_err(|e| format!("Authors fallback failed: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(query::AuthorOption {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    account: row.get("account")?,
                    count: row.get("count")?,
                })
            })
            .map_err(|e| format!("Authors query failed: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Authors collect failed: {e}"))
    }

    fn fallback_tags(conn: &rusqlite::Connection) -> Result<Vec<query::TagOption>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT t.name, t.translated_name, \
                 COUNT(DISTINCT it.image_id) as count \
                 FROM tags t \
                 JOIN image_tags it ON it.tag_id = t.id \
                 GROUP BY t.name ORDER BY count DESC LIMIT 200",
            )
            .map_err(|e| format!("Tags fallback failed: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(query::TagOption {
                    name: row.get("name")?,
                    translated_name: row.get("translated_name")?,
                    count: row.get("count")?,
                })
            })
            .map_err(|e| format!("Tags query failed: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Tags collect failed: {e}"))
    }

    Ok(query::FilterOptions {
        years: read_json(&conn, "sidebar_years", fallback_years)?,
        authors: read_json(&conn, "sidebar_authors", fallback_authors)?,
        tags: read_json(&conn, "sidebar_tags", fallback_tags)?,
    })
}

/// Prefix-search tags for autocomplete. Returns top 15 matching tags
/// ordered by frequency (most images first).
#[tauri::command]
fn search_tags(query: String) -> Result<Vec<query::TagSuggestion>, String> {
    let conn = db::get_conn()?;
    let pattern = format!("{}%", query);
    let mut stmt = conn
        .prepare(
            "SELECT t.name, t.translated_name, \
             COUNT(DISTINCT it.image_id) as count \
             FROM tags t \
             JOIN image_tags it ON it.tag_id = t.id \
             WHERE t.name LIKE ?1 OR t.translated_name LIKE ?1 \
             GROUP BY t.name \
             ORDER BY count DESC \
             LIMIT 15",
        )
        .map_err(|e| format!("Prepare search_tags failed: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![pattern], |row| {
            Ok(query::TagSuggestion {
                name: row.get("name")?,
                translated_name: row.get("translated_name")?,
                count: row.get("count")?,
            })
        })
        .map_err(|e| format!("search_tags query failed: {e}"))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("search_tags row failed: {e}"))?);
    }
    Ok(result)
}

/// Import image data from a raw JSON string into the SQLite database.
/// Deduplicates on (id, part) primary key.
/// Emits `import-progress` events to the frontend during the import.
#[tauri::command]
fn import_json_to_db(
    app: tauri::AppHandle,
    img_dir: String,
    json_content: String,
) -> Result<db::ImportResult, String> {
    let emit = |current: usize, total: usize| {
        let _ = app.emit_all(
            "import-progress",
            serde_json::json!({ "current": current, "total": total }),
        );
    };
    db::import_json_to_db_logic(&img_dir, &json_content, Some(&emit))
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);

            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            start_local_server,
            get_executable_dir,
            restart_app,
            ensure_db,
            query_images,
            query_image_counts,
            get_full_counts,
            refresh_caches,
            reimport_db,
            get_filter_options,
            search_tags,
            import_json_to_db,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
