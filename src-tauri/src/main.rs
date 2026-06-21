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
#[tauri::command]
fn ensure_db(img_dir: String, version: Option<i64>) -> Result<db::EnsureDbResult, String> {
    db::ensure_db(&img_dir, version)
}

/// Paginated, filtered image query against the SQLite database.
/// Supports all filter dimensions from the original JS imageFilter().
#[tauri::command]
fn query_images(query: query::ImageQuery) -> Result<query::QueryResult, String> {
    let conn = db::get_conn()?;

    let (count_sql, count_params) = query.build_count_sql();
    let total: i64 = {
        let refs: Vec<&dyn ToSql> = count_params.iter().map(|b| b.as_ref()).collect();
        conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(&refs),
            |row| row.get(0),
        )
        .map_err(|e| format!("Count query failed: {e}"))?
    };

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
    let mut tag_map: std::collections::HashMap<i64, Vec<query::ImageTag>> =
        std::collections::HashMap::new();
    {
        let ids: Vec<i64> = images.iter().map(|img| img.id).collect();
        if !ids.is_empty() {
            let ph: Vec<String> = (1..=ids.len()).map(|i| format!("?{i}")).collect();
            let tag_sql = format!(
                "SELECT it.image_id, t.name, t.translated_name \
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
                    let name: String = row.get("name")?;
                    let translated_name: Option<String> = row.get("translated_name")?;
                    Ok((image_id, query::ImageTag { name, translated_name }))
                })
                .map_err(|e| format!("Tag query failed: {e}"))?;
            for tag_row in tag_rows {
                let (image_id, tag) = tag_row.map_err(|e| format!("Tag row failed: {e}"))?;
                tag_map.entry(image_id).or_default().push(tag);
            }
        }
    }
    let images: Vec<query::ImageRow> = images
        .into_iter()
        .map(|mut img| {
            if let Some(tags) = tag_map.remove(&img.id) {
                img.tags = tags;
            }
            img
        })
        .collect();

    let (illust_count, author_count, tag_count) =
        if query.include_counts.unwrap_or(false) {
            let (csql, cparams) = query.build_counts_sql();
            let refs: Vec<&dyn ToSql> = cparams.iter().map(|b| b.as_ref()).collect();
            let counts = conn
                .query_row(
                    &csql,
                    rusqlite::params_from_iter(&refs),
                    |row| {
                        Ok((
                            row.get::<_, i64>("illust_count")?,
                            row.get::<_, i64>("author_count")?,
                        ))
                    },
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

            (Some(counts.0), Some(counts.1), Some(tag_count))
        } else {
            (None, None, None)
        };

    Ok(query::QueryResult {
        images,
        total,
        illust_count,
        author_count,
        tag_count,
    })
}

/// Aggregate filter options (years, authors, tags with counts) for the sidebar.
#[tauri::command]
fn get_filter_options() -> Result<query::FilterOptions, String> {
    let conn = db::get_conn()?;

    let years: Vec<query::YearOption> = {
        let mut stmt = conn
            .prepare(
                "SELECT CAST(substr(created_at,1,4) AS INTEGER) as year, \
                 COUNT(*) as count FROM images \
                 GROUP BY year ORDER BY year DESC",
            )
            .map_err(|e| format!("Prepare years failed: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(query::YearOption {
                    year: row.get("year")?,
                    count: row.get("count")?,
                })
            })
            .map_err(|e| format!("Years query failed: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Years collect failed: {e}"))?
    };

    let authors: Vec<query::AuthorOption> = {
        let mut stmt = conn
            .prepare(
                "SELECT author_id as id, author_name as name, \
                 author_account as account, COUNT(DISTINCT id) as count \
                 FROM images GROUP BY author_id \
                 ORDER BY count DESC LIMIT 100",
            )
            .map_err(|e| format!("Prepare authors failed: {e}"))?;
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
            .map_err(|e| format!("Authors collect failed: {e}"))?
    };

    let tags: Vec<query::TagOption> = {
        let mut stmt = conn
            .prepare(
                "SELECT t.name, t.translated_name, \
                 COUNT(DISTINCT it.image_id) as count \
                 FROM tags t \
                 JOIN image_tags it ON it.tag_id = t.id \
                 GROUP BY t.name ORDER BY count DESC LIMIT 200",
            )
            .map_err(|e| format!("Prepare tags failed: {e}"))?;
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
            .map_err(|e| format!("Tags collect failed: {e}"))?
    };

    Ok(query::FilterOptions {
        years,
        authors,
        tags,
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
             WHERE t.name LIKE ?1 \
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
#[tauri::command]
fn import_json_to_db(
    img_dir: String,
    json_content: String,
) -> Result<db::ImportResult, String> {
    db::import_json_to_db_logic(&img_dir, &json_content)
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
            get_filter_options,
            search_tags,
            import_json_to_db,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
