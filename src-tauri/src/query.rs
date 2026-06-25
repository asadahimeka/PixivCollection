//! Serializable query types and SQL filter builder for the PixivCollection
//! SQLite database.
//!
//! Each [`ImageQuery`] is decoded from a Tauri command argument and used to
//! build parameterised SQL queries.  No raw string interpolation – all
//! user-supplied values go through `?N` placeholders.

use rusqlite::types::ToSql;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Query parameter struct – deserialised from Tauri command args
// ---------------------------------------------------------------------------

/// Filter, sort and pagination parameters for the image gallery query.
#[derive(Debug, Deserialize)]
pub struct ImageQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    /// `"original"` (default), `"id_asc"`, `"id_desc"`, or `"bookmark_desc"`
    pub sort_by: Option<String>,
    /// Free-text search (matched against id, title, author id, author name,
    /// tag names and translated tag names).
    pub search: Option<String>,
    /// Year filter: `1` = "before 2000", otherwise exact year match.
    pub year: Option<i64>,
    /// Exact tag name filter.
    pub tag: Option<String>,
    pub author_id: Option<i64>,
    /// `"horizontal"`, `"vertical"`, `"square"`, `"ratio-4:3"`,
    /// `"ratio-16:9"`, etc.
    pub shape: Option<String>,
    pub width_min: Option<i64>,
    pub width_max: Option<i64>,
    pub height_min: Option<i64>,
    pub height_max: Option<i64>,
    /// Bookmark count lower bound.  `-1` selects unbookmarked images only.
    pub bookmark_min: Option<i64>,
    /// `"hidden"` (hide R18), `"only"` (R18 only), or `"show"` (no filter).
    pub r18: Option<String>,
    pub max_sanity_level: Option<i64>,
    /// Stable seed for deterministic random ordering.
    /// When `sort_by = "random"` and a seed is provided, uses
    /// `(id * seed + part * 7919) % 2147483647` instead of `RANDOM()`
    /// so that pagination yields consistent, non-overlapping results.
    pub random_seed: Option<i64>,
}

// ---------------------------------------------------------------------------
// Response structs – serialised to JSON for the Tauri frontend
// ---------------------------------------------------------------------------

/// A single tag attached to an image.
#[derive(Debug, Serialize, Clone)]
pub struct ImageTag {
    pub name: String,
    pub translated_name: Option<String>,
}

/// A single image row matching the frontend `Image` type.
#[derive(Debug, Serialize)]
pub struct ImageRow {
    pub id: i64,
    pub part: i64,
    pub len: i64,
    pub title: String,
    pub width: i64,
    pub height: i64,
    pub ext: String,
    pub author_id: i64,
    pub author_name: String,
    pub author_account: String,
    pub bookmark: i64,
    pub view: i64,
    pub created_at: String,
    pub sanity_level: i64,
    pub x_restrict: i64,
    pub is_ai: bool,
    pub img_s: String,
    pub img_m: String,
    pub img_l: String,
    pub img_o: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<ImageTag>,
}

/// Paginated query result.
#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub images: Vec<ImageRow>,
}

/// Aggregate counts result returned by the `query_image_counts` command.
#[derive(Debug, Serialize)]
pub struct CountsResult {
    pub total: i64,
    pub illust_count: i64,
    pub author_count: i64,
    pub tag_count: i64,
}

/// Grouped filter options (years / authors / tags) for the filter sidebar.
#[derive(Debug, Serialize)]
pub struct FilterOptions {
    pub years: Vec<YearOption>,
    pub authors: Vec<AuthorOption>,
    pub tags: Vec<TagOption>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YearOption {
    pub year: i64,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorOption {
    pub id: i64,
    pub name: String,
    pub account: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagOption {
    pub name: String,
    pub translated_name: Option<String>,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct TagSuggestion {
    pub name: String,
    pub translated_name: Option<String>,
    pub count: i64,
}

// ---------------------------------------------------------------------------
// Helper – push a parameter and return its `?N` index
// ---------------------------------------------------------------------------

fn push_param<T>(idx: &mut u32, params: &mut Vec<Box<dyn ToSql>>, val: T) -> u32
where
    T: ToSql + 'static,
{
    *idx += 1;
    params.push(Box::new(val));
    *idx
}

// ---------------------------------------------------------------------------
// SQL fragments (shared constants)
// ---------------------------------------------------------------------------

const BASE_SELECT: &str = concat!(
    "SELECT i.id, i.part, i.len, i.title, i.width, i.height, i.ext, ",
    "i.author_id, i.author_name, i.author_account, i.bookmark, i.view, ",
    "i.created_at, i.sanity_level, i.x_restrict, i.is_ai, ",
    "i.img_s, i.img_m, i.img_l, i.img_o FROM images i WHERE 1=1",
);

const COUNTS_SELECT: &str = concat!(
    "SELECT COUNT(*) as total, ",
    "COUNT(DISTINCT i.id) as illust_count, ",
    "COUNT(DISTINCT i.author_id) as author_count ",
    "FROM images i WHERE 1=1",
);

const TAG_COUNT_SELECT: &str = concat!(
    "SELECT COUNT(DISTINCT it.tag_id) FROM images i ",
    "JOIN image_tags it ON i.id = it.image_id AND i.part = it.image_part ",
    "WHERE 1=1",
);

// ---------------------------------------------------------------------------
// SQL builders
// ---------------------------------------------------------------------------

impl ImageQuery {
    /// Build the shared `WHERE` clause (without ORDER BY / LIMIT / OFFSET).
    ///
    /// Returns `(sql_fragment, params)` where `sql_fragment` is the part
    /// appended after `WHERE 1=1`, e.g. `" AND i.x_restrict < 1 AND …"`.
    fn build_where(&self) -> (String, Vec<Box<dyn ToSql>>) {
        let mut sql = String::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let mut pi = 0u32;

        // ---- R18 ----
        if let Some(ref val) = self.r18 {
            match val.as_str() {
                "hidden" => sql.push_str(" AND i.x_restrict < 1"),
                "only" => sql.push_str(" AND i.x_restrict >= 1"),
                _ => { /* "show" or unknown – no filter */ }
            }
        }

        // ---- Max sanity level ----
        if let Some(sl) = self.max_sanity_level {
            let n = push_param(&mut pi, &mut params, sl);
            sql.push_str(&format!(" AND i.sanity_level <= ?{}", n));
        }

        // ---- Search (multi-field case-insensitive) ----
        if let Some(ref term) = self.search {
            let pattern = format!("%{}%", term.to_lowercase());
            let n = push_param(&mut pi, &mut params, pattern);
            sql.push_str(&format!(
                " AND (CAST(i.id AS TEXT) LIKE ?{} \
                 OR i.title LIKE ?{} \
                 OR CAST(i.author_id AS TEXT) LIKE ?{} \
                 OR i.author_name LIKE ?{} \
                 OR EXISTS( \
                     SELECT 1 FROM image_tags it \
                     JOIN tags t ON it.tag_id = t.id \
                     WHERE it.image_id = i.id \
                       AND it.image_part = i.part \
                       AND (LOWER(t.name) LIKE LOWER(?{}) \
                         OR LOWER(t.translated_name) LIKE LOWER(?{}) \
                       ) \
                   ))",
                n, n, n, n, n, n,
            ));
        }

        // ---- Bookmark ----
        if let Some(bm) = self.bookmark_min {
            if bm == -1 {
                sql.push_str(" AND i.bookmark = -1");
            } else if bm >= 0 {
                let n = push_param(&mut pi, &mut params, bm);
                sql.push_str(&format!(" AND i.bookmark >= ?{}", n));
            }
        }

        // ---- Year (range comparison for index usage) ----
        if let Some(y) = self.year {
            if y == 1 {
                sql.push_str(" AND i.created_at < '2000-01-01'");
            } else if y > 1 {
                let year_start = format!("{}-01-01", y);
                let year_end = format!("{}-01-01", y + 1);
                let n1 = push_param(&mut pi, &mut params, year_start);
                let n2 = push_param(&mut pi, &mut params, year_end);
                sql.push_str(&format!(
                    " AND i.created_at >= ?{} AND i.created_at < ?{}",
                    n1, n2,
                ));
            }
        }

        // ---- Tag (exact name match) ----
        if let Some(ref t) = self.tag {
            let n = push_param(&mut pi, &mut params, t.clone());
            sql.push_str(&format!(
                " AND EXISTS( \
                     SELECT 1 FROM image_tags it \
                     JOIN tags t ON it.tag_id = t.id \
                     WHERE it.image_id = i.id \
                       AND it.image_part = i.part \
                       AND t.name = ?{} \
                   )",
                n,
            ));
        }

        // ---- Author ----
        if let Some(aid) = self.author_id {
            let n = push_param(&mut pi, &mut params, aid);
            sql.push_str(&format!(" AND i.author_id = ?{}", n));
        }

        // ---- Shape ----
        if let Some(ref s) = self.shape {
            match s.as_str() {
                "horizontal" => {
                    sql.push_str(" AND i.width > i.height * 1.1");
                }
                "vertical" => {
                    sql.push_str(" AND i.width < i.height * 0.9");
                }
                "square" => {
                    sql.push_str(
                        " AND i.width BETWEEN i.height * 0.9 AND i.height * 1.1",
                    );
                }
                "ratio-4:3" => {
                    sql.push_str(
                        " AND ABS(i.width * 3 - i.height * 4) * 100 \
                         <= (i.width * 3 + i.height * 4) / 2",
                    );
                }
                "ratio-16:9" => {
                    sql.push_str(
                        " AND ABS(i.width * 9 - i.height * 16) * 100 \
                         <= (i.width * 9 + i.height * 16) / 2",
                    );
                }
                "ratio-21:9" => {
                    sql.push_str(
                        " AND ABS(i.width * 9 - i.height * 21) * 100 \
                         <= (i.width * 9 + i.height * 21) / 2",
                    );
                }
                "ratio-3:4" => {
                    sql.push_str(
                        " AND ABS(i.width * 4 - i.height * 3) * 100 \
                         <= (i.width * 4 + i.height * 3) / 2",
                    );
                }
                "ratio-9:16" => {
                    sql.push_str(
                        " AND ABS(i.width * 16 - i.height * 9) * 100 \
                         <= (i.width * 16 + i.height * 9) / 2",
                    );
                }
                "ratio-9:21" => {
                    sql.push_str(
                        " AND ABS(i.width * 21 - i.height * 9) * 100 \
                         <= (i.width * 21 + i.height * 9) / 2",
                    );
                }
                _ => { /* unknown shape – skip */ }
            }
        }

        // ---- Width range ----
        if let Some(w) = self.width_min {
            let n = push_param(&mut pi, &mut params, w);
            sql.push_str(&format!(" AND i.width >= ?{}", n));
        }
        if let Some(w) = self.width_max {
            let n = push_param(&mut pi, &mut params, w);
            sql.push_str(&format!(" AND i.width <= ?{}", n));
        }

        // ---- Height range ----
        if let Some(h) = self.height_min {
            let n = push_param(&mut pi, &mut params, h);
            sql.push_str(&format!(" AND i.height >= ?{}", n));
        }
        if let Some(h) = self.height_max {
            let n = push_param(&mut pi, &mut params, h);
            sql.push_str(&format!(" AND i.height <= ?{}", n));
        }

        (sql, params)
    }

    /// Build a paginated `SELECT` with ORDER BY, LIMIT, and OFFSET.
    ///
    /// Callers should execute the returned SQL with `rusqlite::params_from_iter`
    /// after mapping the `Vec<Box<dyn ToSql>>` to `&dyn ToSql` references:
    ///
    /// ```ignore
    /// let (sql, params) = query.build_sql();
    /// let mut stmt = conn.prepare(&sql)?;
    /// let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
    /// let rows = stmt.query(rusqlite::params_from_iter(refs))?;
    /// ```
    pub fn build_sql(&self) -> (String, Vec<Box<dyn ToSql>>) {
        let (where_clause, mut params) = self.build_where();
        let mut pi = params.len() as u32;
        let mut sql = String::from(BASE_SELECT);
        sql.push_str(&where_clause);

        // ORDER BY
        match self.sort_by.as_deref().unwrap_or("original") {
            "id_asc" => sql.push_str(" ORDER BY i.id ASC, i.part ASC"),
            "id_desc" => sql.push_str(" ORDER BY i.id DESC, i.part ASC"),
            "created_at_asc" => {
                sql.push_str(" ORDER BY i.created_at ASC, i.id ASC, i.part ASC");
            }
            "created_at_desc" => {
                sql.push_str(" ORDER BY i.created_at DESC, i.id DESC, i.part ASC");
            }
            "bookmark_asc" => {
                sql.push_str(" ORDER BY i.bookmark ASC, i.id ASC, i.part ASC");
            }
            "bookmark_desc" => {
                sql.push_str(" ORDER BY i.bookmark DESC, i.id DESC, i.part ASC");
            }
            "view_asc" => {
                sql.push_str(" ORDER BY i.view ASC, i.id ASC, i.part ASC");
            }
            "view_desc" => {
                sql.push_str(" ORDER BY i.view DESC, i.id DESC, i.part ASC");
            }
            "random" => {
                if let Some(seed) = self.random_seed {
                    let n = push_param(&mut pi, &mut params, seed);
                    sql.push_str(&format!(
                        " ORDER BY (i.id * ?{}) % 2147483647, i.part ASC",
                        n,
                    ));
                } else {
                    sql.push_str(" ORDER BY RANDOM()");
                }
            }
            _ => sql.push_str(" ORDER BY i.img_order ASC, i.part ASC"),
        }

        // LIMIT / OFFSET
        let limit = self.limit.unwrap_or(60);
        let offset = self.offset.unwrap_or(0);
        let n_lim = push_param(&mut pi, &mut params, limit);
        let n_off = push_param(&mut pi, &mut params, offset);
        sql.push_str(&format!(" LIMIT ?{} OFFSET ?{}", n_lim, n_off));

        (sql, params)
    }

    /// Build a multi-count query returning `total`, `illust_count`, and
    /// `author_count` using the same filters.
    pub fn build_counts_sql(&self) -> (String, Vec<Box<dyn ToSql>>) {
        let (where_clause, params) = self.build_where();
        let mut sql = String::from(COUNTS_SELECT);
        sql.push_str(&where_clause);
        (sql, params)
    }

    /// Build a `COUNT(DISTINCT t.name)` query for tag count using the same
    /// filters.
    pub fn build_tag_count_sql(&self) -> (String, Vec<Box<dyn ToSql>>) {
        let (where_clause, params) = self.build_where();
        let mut sql = String::from(TAG_COUNT_SELECT);
        sql.push_str(&where_clause);
        (sql, params)
    }
}

// ---------------------------------------------------------------------------
// Row mapping
// ---------------------------------------------------------------------------

/// Map a [`rusqlite::Row`] to an [`ImageRow`].
///
/// Column names must match the `SELECT` list used in [`BASE_SELECT`].
pub fn image_row_from_row(row: &rusqlite::Row) -> rusqlite::Result<ImageRow> {
    Ok(ImageRow {
        id: row.get("id")?,
        part: row.get("part")?,
        len: row.get("len")?,
        title: row.get("title")?,
        width: row.get("width")?,
        height: row.get("height")?,
        ext: row.get("ext")?,
        author_id: row.get("author_id")?,
        author_name: row.get("author_name")?,
        author_account: row.get("author_account")?,
        bookmark: row.get("bookmark")?,
        view: row.get("view")?,
        created_at: row.get("created_at")?,
        sanity_level: row.get("sanity_level")?,
        x_restrict: row.get("x_restrict")?,
        is_ai: row.get("is_ai")?,
        img_s: row.get("img_s")?,
        img_m: row.get("img_m")?,
        img_l: row.get("img_l")?,
        img_o: row.get("img_o")?,
        tags: Vec::new(),
    })
}
