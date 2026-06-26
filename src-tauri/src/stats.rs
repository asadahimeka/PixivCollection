//! Bookmark statistics computation for the PixivCollection SQLite database.
//!
//! Provides [`StatsFilter`] for filtering the dataset and
//! [`compute_statistics`] for computing all statistical aggregations
//! exposed to the Tauri frontend.
//!
//! # Filtering
//!
//! Every query uses `{where}` as a placeholder for the shared WHERE clause
//! built by [`StatsFilter::build_where_clause`].  Parameterised `?N`
//! placeholders are used throughout – no raw string interpolation of
//! user-supplied values.

use rusqlite::types::ToSql;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Top-level result
// ---------------------------------------------------------------------------

/// All bookmark statistics in one structure.
#[derive(Debug, Serialize)]
pub struct StatisticsResult {
    pub overview: StatsOverview,
    pub author_ranking: Vec<AuthorStats>,
    pub tag_ranking: Vec<TagStats>,
    pub bookmark_distribution: Vec<DistributionBucket>,
    pub view_distribution: Vec<DistributionBucket>,
    pub yearly_trend: Vec<YearlyStats>,
    pub r18_ratio: R18Ratio,
    pub ai_ratio: AiRatio,
    pub shape_distribution: Vec<ShapeBucket>,
    pub top_bookmarked: Vec<TopWork>,
    pub top_viewed: Vec<TopWork>,
    pub author_works_distribution: Vec<AuthorBucket>,
    pub r18_trend: Vec<R18TrendItem>,
    pub ai_trend: Vec<AiTrendItem>,
    pub tag_trend: Vec<TagTrendItem>,
    pub hidden_gems: Vec<HiddenGem>,
    pub author_discovery: Vec<AuthorDiscovery>,
    pub sanity_levels: Vec<SanityLevelBucket>,
}

// ---------------------------------------------------------------------------
// Struct definitions
// ---------------------------------------------------------------------------

/// High-level counts and year range.
#[derive(Debug, Serialize)]
pub struct StatsOverview {
    pub total_illustrations: i64,
    pub total_authors: i64,
    pub total_tags: i64,
    pub year_range: (i32, i32),
}

/// Per-author aggregate statistics.
#[derive(Debug, Serialize)]
pub struct AuthorStats {
    pub author_id: i64,
    pub author_name: String,
    pub author_account: String,
    pub illustration_count: i64,
    pub total_bookmarks: i64,
    pub total_views: i64,
}

/// Per-tag aggregate statistics.
#[derive(Debug, Serialize)]
pub struct TagStats {
    pub name: String,
    pub translated_name: Option<String>,
    pub count: i64,
}

/// A single bucket in a value distribution (bookmarks / views).
#[derive(Debug, Serialize)]
pub struct DistributionBucket {
    pub label: String,
    pub min: i64,
    pub max: Option<i64>,
    pub count: i64,
}

/// Per-year aggregate statistics.
#[derive(Debug, Serialize)]
pub struct YearlyStats {
    pub year: i32,
    pub count: i64,
    pub avg_bookmark: f64,
    pub avg_view: f64,
}

/// R18 content counts.
#[derive(Debug, Serialize)]
pub struct R18Ratio {
    pub safe: i64,
    pub r18: i64,
    pub r18g: i64,
}

/// AI / non-AI content counts.
#[derive(Debug, Serialize)]
pub struct AiRatio {
    pub ai: i64,
    pub non_ai: i64,
}

/// Image aspect-ratio / shape bucket (per-page count).
#[derive(Debug, Serialize)]
pub struct ShapeBucket {
    pub shape: String,
    pub count: i64,
}

/// A single top bookmarked or top viewed work.
#[derive(Debug, Serialize)]
pub struct TopWork {
    pub id: i64,
    pub title: String,
    pub value: i64,
    pub author_name: String,
}

/// Author productivity distribution bucket.
#[derive(Debug, Serialize)]
pub struct AuthorBucket {
    pub label: String,
    pub count: i64,
}

/// R18 trend item for a single year and restrict level.
#[derive(Debug, Serialize)]
pub struct R18TrendItem {
    pub year: i32,
    pub x_restrict: i32,
    pub count: i64,
}

/// AI trend item for a single year.
#[derive(Debug, Serialize)]
pub struct AiTrendItem {
    pub year: i32,
    pub is_ai: bool,
    pub count: i64,
}

/// Tag popularity trend for a single year.
#[derive(Debug, Serialize)]
pub struct TagTrendItem {
    pub year: i32,
    pub tag_name: String,
    pub translated_name: Option<String>,
    pub count: i64,
}

/// A hidden gem – high bookmark-to-view ratio.
#[derive(Debug, Serialize)]
pub struct HiddenGem {
    pub id: i64,
    pub title: String,
    pub author_name: String,
    pub bookmark: i64,
    pub view: i64,
    pub ratio: f64,
}

/// Author discovery – first appearance year.
#[derive(Debug, Serialize)]
pub struct AuthorDiscovery {
    pub author_id: i64,
    pub author_name: String,
    pub author_account: String,
    pub first_year: i32,
    pub works_count: i64,
}

/// Sanity level distribution bucket.
#[derive(Debug, Serialize)]
pub struct SanityLevelBucket {
    pub level: i32,
    pub count: i64,
}

// ---------------------------------------------------------------------------
// Filter
// ---------------------------------------------------------------------------

/// Filter constraints applied to every statistics query.
///
/// Each field is optional – `None` means no restriction for that dimension.
/// The `build_where_clause` method returns a parameterised SQL fragment and
/// the bound parameter list.
#[derive(Debug, Default)]
pub struct StatsFilter {
    pub year_min: Option<i32>,
    pub year_max: Option<i32>,
    /// `"show"` = no filter, `"hidden"` = safe only, `"only"` = R18/R18G only.
    pub r18: Option<String>,
    pub is_ai: Option<bool>,
}

impl StatsFilter {
    /// Build the shared WHERE clause and parameter list.
    ///
    /// Returns `(sql_fragment, params)` where `sql_fragment` starts with
    /// `"1=1"` (no-filter base case) and may have ` AND …` appended.
    ///
    /// # Examples
    ///
    /// ```
    /// let f = StatsFilter::default();
    /// let (sql, params) = f.build_where_clause();
    /// assert_eq!(sql, "1=1");
    /// assert!(params.is_empty());
    /// ```
    pub fn build_where_clause(&self) -> (String, Vec<Box<dyn ToSql>>) {
        let mut clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        let mut pi = 0u32;

        // ---- Year range ----
        if self.year_min.is_some() || self.year_max.is_some() {
            let y_min = self.year_min.unwrap_or(0);
            let y_max = self.year_max.unwrap_or(9_999);
            pi += 1;
            params.push(Box::new(y_min));
            pi += 1;
            params.push(Box::new(y_max));
            clauses.push(format!(
                "CAST(substr(i.created_at,1,4) AS INTEGER) BETWEEN ?{} AND ?{}",
                pi - 1,
                pi,
            ));
        }

        // ---- R18 ----
        if let Some(ref val) = self.r18 {
            match val.as_str() {
                "hidden" => {
                    pi += 1;
                    params.push(Box::new(0i64));
                    clauses.push(format!("i.x_restrict = ?{}", pi));
                }
                "only" => {
                    pi += 1;
                    params.push(Box::new(0i64));
                    clauses.push(format!("i.x_restrict > ?{}", pi));
                }
                _ => { /* "show" or unknown – no filter */ }
            }
        }

        // ---- AI ----
        if let Some(ai) = self.is_ai {
            pi += 1;
            params.push(Box::new(ai));
            clauses.push(format!("i.is_ai = ?{}", pi));
        }

        let where_clause = if clauses.is_empty() {
            "1=1".to_string()
        } else {
            clauses.join(" AND ")
        };

        (where_clause, params)
    }
}

// ---------------------------------------------------------------------------
// Helper – map a bookmark/view distribution label to its numeric bounds
// ---------------------------------------------------------------------------

/// Parse a distribution bucket label (e.g. `"1000-4999"` or `"10000+")`
/// into its lower and upper bounds.
fn parse_bucket_bounds(label: &str) -> (i64, Option<i64>) {
    if let Some(plus) = label.strip_suffix('+') {
        // Unbounded upper: "10000+" → 10000, None
        let min: i64 = plus.parse().unwrap_or(0);
        (min, None)
    } else if let Some(dash) = label.find('-') {
        let min: i64 = label[..dash].parse().unwrap_or(0);
        let max: i64 = label[dash + 1..].parse().unwrap_or(0);
        (min, Some(max))
    } else {
        // Single-value label (shouldn't happen, but be safe)
        let v: i64 = label.parse().unwrap_or(0);
        (v, Some(v))
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute all bookmark statistics for the given filter.
///
/// Executes a series of parameterised SQL queries against `conn`, reusing the
/// same WHERE clause and parameter list from `filter.build_where_clause()` so
/// that all aggregations operate on the same filtered subset.
pub fn compute_statistics(
    conn: &rusqlite::Connection,
    filter: &StatsFilter,
) -> Result<StatisticsResult, String> {
    let (where_clause, params) = filter.build_where_clause();

    // ---- 1. Overview ----
    // NOTE: total_tags needs distinct tag_ids from image_tags, hence the
    // LEFT JOIN with a distinct subquery to avoid inflating the count.
    let overview_sql = format!(
        "SELECT COUNT(DISTINCT i.id), COUNT(DISTINCT i.author_id), \
                COUNT(DISTINCT t_count.tag_id), \
                MIN(CAST(substr(i.created_at,1,4) AS INTEGER)), \
                MAX(CAST(substr(i.created_at,1,4) AS INTEGER)) \
         FROM images i \
         LEFT JOIN (SELECT DISTINCT it.image_id, it.tag_id FROM image_tags it) t_count \
           ON t_count.image_id = i.id \
         WHERE {}",
        where_clause,
    );
    let overview = {
        let mut stmt = conn
            .prepare(&overview_sql)
            .map_err(|e| format!("Overview prepare: {e}"))?;
        let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        stmt.query_row(rusqlite::params_from_iter(&refs), |row| {
            let min_year: Option<i32> = row.get(3)?;
            let max_year: Option<i32> = row.get(4)?;
            Ok(StatsOverview {
                total_illustrations: row.get(0)?,
                total_authors: row.get(1)?,
                total_tags: row.get(2)?,
                year_range: (min_year.unwrap_or(0), max_year.unwrap_or(0)),
            })
        })
        .map_err(|e| format!("Overview query: {e}"))?
    };

    // ---- Helper to run a multi-row aggregate query ----
    // (avoids repeating the prepare / params_from_iter dance for every
    //  section that returns a Vec of simple types)
    macro_rules! query_vec {
        ($sql_expr:expr, $mapper:expr) => {{
            let sql = $sql_expr;
            let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare: {e}"))?;
            let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
            let rows = stmt
                .query_map(rusqlite::params_from_iter(&refs), $mapper)
                .map_err(|e| format!("Query: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Collect: {e}"))?
        }};
    }

    // ---- 2. Author Ranking (TOP 200) ----
    let author_ranking_sql = format!(
        "SELECT i.author_id, i.author_name, i.author_account, \
                COUNT(DISTINCT i.id) as illust_count, \
                SUM(i.bookmark) as total_bookmarks, \
                SUM(i.view) as total_views \
         FROM images i WHERE {} \
         GROUP BY i.author_id ORDER BY illust_count DESC LIMIT 200",
        where_clause,
    );
    let author_ranking: Vec<AuthorStats> = query_vec!(
        author_ranking_sql,
        |row| {
            Ok(AuthorStats {
                author_id: row.get("author_id")?,
                author_name: row.get("author_name")?,
                author_account: row.get("author_account")?,
                illustration_count: row.get("illust_count")?,
                total_bookmarks: row.get("total_bookmarks")?,
                total_views: row.get("total_views")?,
            })
        }
    );

    // ---- 3. Tag Ranking (TOP 200) ----
    let tag_ranking_sql = format!(
        "SELECT t.name, t.translated_name, COUNT(DISTINCT it.image_id) as count \
         FROM tags t \
         JOIN image_tags it ON it.tag_id = t.id \
         JOIN images i ON i.id = it.image_id AND i.part = it.image_part \
         WHERE {} \
         GROUP BY t.id ORDER BY count DESC LIMIT 200",
        where_clause,
    );
    let tag_ranking: Vec<TagStats> = query_vec!(
        tag_ranking_sql,
        |row| {
            Ok(TagStats {
                name: row.get("name")?,
                translated_name: row.get("translated_name")?,
                count: row.get("count")?,
            })
        }
    );

    // ---- 4. Bookmark Distribution ----
    let bm_dist_sql = format!(
        "SELECT COUNT(DISTINCT i.id) as count, \
                CASE WHEN i.bookmark >= 10000 THEN '10000+' \
                     WHEN i.bookmark >= 5000  THEN '5000-9999' \
                     WHEN i.bookmark >= 1000  THEN '1000-4999' \
                     WHEN i.bookmark >= 100   THEN '100-999' \
                     ELSE '0-99' END as bucket, \
                MIN(i.bookmark) as min_val \
         FROM images i WHERE {} \
         GROUP BY bucket ORDER BY MIN(i.bookmark)",
        where_clause,
    );
    let bookmark_distribution: Vec<DistributionBucket> = {
        let mut stmt = conn
            .prepare(&bm_dist_sql)
            .map_err(|e| format!("Bookmark dist prepare: {e}"))?;
        let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(&refs), |row| {
                let label: String = row.get("bucket")?;
                let (min, max) = parse_bucket_bounds(&label);
                Ok(DistributionBucket {
                    label,
                    min,
                    max,
                    count: row.get("count")?,
                })
            })
            .map_err(|e| format!("Bookmark dist query: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Bookmark dist collect: {e}"))?
    };

    // ---- 5. View Distribution ----
    let view_dist_sql = format!(
        "SELECT COUNT(DISTINCT i.id) as count, \
                CASE WHEN i.view >= 100000 THEN '100000+' \
                     WHEN i.view >= 50000  THEN '50000-99999' \
                     WHEN i.view >= 10000  THEN '10000-49999' \
                     WHEN i.view >= 5000   THEN '5000-9999' \
                     WHEN i.view >= 1000   THEN '1000-4999' \
                     ELSE '0-999' END as bucket, \
                MIN(i.view) as min_val \
         FROM images i WHERE {} \
         GROUP BY bucket ORDER BY MIN(i.view)",
        where_clause,
    );
    let view_distribution: Vec<DistributionBucket> = {
        let mut stmt = conn
            .prepare(&view_dist_sql)
            .map_err(|e| format!("View dist prepare: {e}"))?;
        let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(&refs), |row| {
                let label: String = row.get("bucket")?;
                let (min, max) = parse_bucket_bounds(&label);
                Ok(DistributionBucket {
                    label,
                    min,
                    max,
                    count: row.get("count")?,
                })
            })
            .map_err(|e| format!("View dist query: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("View dist collect: {e}"))?
    };

    // ---- 6. Yearly Trend ----
    let yearly_trend_sql = format!(
        "SELECT CAST(substr(i.created_at,1,4) AS INTEGER) as year, \
                COUNT(DISTINCT i.id) as count, \
                AVG(i.bookmark) as avg_bookmark, \
                AVG(i.view) as avg_view \
         FROM images i WHERE {} \
         GROUP BY year ORDER BY year",
        where_clause,
    );
    let yearly_trend: Vec<YearlyStats> = query_vec!(
        yearly_trend_sql,
        |row| {
            Ok(YearlyStats {
                year: row.get("year")?,
                count: row.get("count")?,
                avg_bookmark: row.get("avg_bookmark")?,
                avg_view: row.get("avg_view")?,
            })
        }
    );

    // ---- 7. R18 Ratio ----
    let r18_ratio_sql = format!(
        "SELECT i.x_restrict, COUNT(DISTINCT i.id) as count \
         FROM images i WHERE {} \
         GROUP BY i.x_restrict",
        where_clause,
    );
    let r18_ratio = {
        let rows: Vec<(i64, i64)> = query_vec!(
            r18_ratio_sql,
            |row| Ok((row.get("x_restrict")?, row.get("count")?))
        );
        let mut safe = 0i64;
        let mut r18 = 0i64;
        let mut r18g = 0i64;
        for (restrict, cnt) in rows {
            match restrict {
                0 => safe += cnt,
                1 => r18 += cnt,
                2 => r18g += cnt,
                _ => {
                    // Unknown restrict level – treat as R18
                    r18 += cnt;
                }
            }
        }
        R18Ratio { safe, r18, r18g }
    };

    // ---- 8. AI Ratio ----
    let ai_ratio_sql = format!(
        "SELECT i.is_ai, COUNT(DISTINCT i.id) as count \
         FROM images i WHERE {} \
         GROUP BY i.is_ai",
        where_clause,
    );
    let ai_ratio = {
        let rows: Vec<(i64, i64)> = query_vec!(
            ai_ratio_sql,
            |row| Ok((row.get("is_ai")?, row.get("count")?))
        );
        let mut ai = 0i64;
        let mut non_ai = 0i64;
        for (is_ai_val, cnt) in rows {
            if is_ai_val != 0 {
                ai += cnt;
            } else {
                non_ai += cnt;
            }
        }
        AiRatio { ai, non_ai }
    };

    // ---- 9. Shape Distribution (per-page COUNT(*)) ----
    let shape_dist_sql = format!(
        "SELECT CASE \
                  WHEN i.width = i.height THEN 'square' \
                  WHEN i.width > i.height \
                   AND CAST(i.width AS REAL) / i.height >= 1.7 THEN '16:9' \
                  WHEN i.width > i.height \
                   AND CAST(i.width AS REAL) / i.height >= 1.3 THEN '4:3' \
                  WHEN i.width > i.height THEN 'horizontal' \
                  WHEN i.height > i.width \
                   AND CAST(i.height AS REAL) / i.width >= 1.7 THEN '9:16' \
                  WHEN i.height > i.width \
                   AND CAST(i.height AS REAL) / i.width >= 1.3 THEN '3:4' \
                  WHEN i.height > i.width THEN 'vertical' \
                  ELSE 'other' END as shape, \
                COUNT(*) as count \
         FROM images i WHERE {} \
         GROUP BY shape ORDER BY count DESC",
        where_clause,
    );
    let shape_distribution: Vec<ShapeBucket> = query_vec!(
        shape_dist_sql,
        |row| {
            Ok(ShapeBucket {
                shape: row.get("shape")?,
                count: row.get("count")?,
            })
        }
    );

    // ---- 10. Top Works (by bookmark, part=0) ----
    let top_bookmarked_sql = format!(
        "SELECT i.id, i.title, i.bookmark as value, i.author_name \
         FROM images i WHERE {} AND i.part = 0 \
         ORDER BY i.bookmark DESC LIMIT 10",
        where_clause,
    );
    let top_bookmarked: Vec<TopWork> = query_vec!(
        top_bookmarked_sql,
        |row| {
            Ok(TopWork {
                id: row.get("id")?,
                title: row.get("title")?,
                value: row.get("value")?,
                author_name: row.get("author_name")?,
            })
        }
    );

    // ---- 11. Top Works (by view, part=0) ----
    let top_viewed_sql = format!(
        "SELECT i.id, i.title, i.view as value, i.author_name \
         FROM images i WHERE {} AND i.part = 0 \
         ORDER BY i.view DESC LIMIT 10",
        where_clause,
    );
    let top_viewed: Vec<TopWork> = query_vec!(
        top_viewed_sql,
        |row| {
            Ok(TopWork {
                id: row.get("id")?,
                title: row.get("title")?,
                value: row.get("value")?,
                author_name: row.get("author_name")?,
            })
        }
    );

    // ---- 12. Author Works Distribution ----
    let author_works_sql = format!(
        "SELECT CASE WHEN cnt = 1 THEN '1' \
                     WHEN cnt <= 5 THEN '2-5' \
                     WHEN cnt <= 10 THEN '6-10' \
                     WHEN cnt <= 20 THEN '11-20' \
                     ELSE '21+' END as bucket, \
                COUNT(*) as count \
         FROM (SELECT i.author_id, COUNT(DISTINCT i.id) as cnt \
               FROM images i WHERE {} \
               GROUP BY i.author_id) sub \
         GROUP BY bucket ORDER BY MIN(cnt)",
        where_clause,
    );
    let author_works_distribution: Vec<AuthorBucket> = query_vec!(
        author_works_sql,
        |row| {
            Ok(AuthorBucket {
                label: row.get("bucket")?,
                count: row.get("count")?,
            })
        }
    );

    // ---- 13. R18 Trend by Year ----
    let r18_trend_sql = format!(
        "SELECT CAST(substr(i.created_at,1,4) AS INTEGER) as year, \
                i.x_restrict, \
                COUNT(DISTINCT i.id) as count \
         FROM images i WHERE {} \
         GROUP BY year, x_restrict ORDER BY year, x_restrict",
        where_clause,
    );
    let r18_trend: Vec<R18TrendItem> = query_vec!(
        r18_trend_sql,
        |row| {
            Ok(R18TrendItem {
                year: row.get("year")?,
                x_restrict: row.get::<_, i64>("x_restrict")? as i32,
                count: row.get("count")?,
            })
        }
    );

    // ---- 14. AI Trend by Year ----
    let ai_trend_sql = format!(
        "SELECT CAST(substr(i.created_at,1,4) AS INTEGER) as year, \
                i.is_ai, \
                COUNT(DISTINCT i.id) as count \
         FROM images i WHERE {} \
         GROUP BY year, is_ai ORDER BY year, is_ai",
        where_clause,
    );
    let ai_trend: Vec<AiTrendItem> = query_vec!(
        ai_trend_sql,
        |row| {
            Ok(AiTrendItem {
                year: row.get("year")?,
                is_ai: row.get::<_, i64>("is_ai")? != 0,
                count: row.get("count")?,
            })
        }
    );

    // ---- 15. Tag Trend (Top 20 tags yearly) ----
    let tag_trend = {
        // Step 1: Find TOP 20 tags within the filtered dataset
        let top_tags_sql = format!(
            "SELECT t.name, COUNT(DISTINCT it.image_id) as cnt \
             FROM tags t \
             JOIN image_tags it ON it.tag_id = t.id \
             JOIN images i ON i.id = it.image_id AND i.part = it.image_part \
             WHERE {} \
             GROUP BY t.id ORDER BY cnt DESC LIMIT 20",
            where_clause,
        );
        let top_tag_names: Vec<String> = {
            let mut stmt = conn
                .prepare(&top_tags_sql)
                .map_err(|e| format!("Top tags prepare: {e}"))?;
            let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
            let rows = stmt
                .query_map(rusqlite::params_from_iter(&refs), |row| row.get::<_, String>(0))
                .map_err(|e| format!("Top tags query: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Top tags collect: {e}"))?
        };

        if top_tag_names.is_empty() {
            Vec::new()
        } else {
            // Step 2: Build a fresh parameterised query for yearly breakdown.
            // We build a new WHERE clause from the filter fields using
            // unnumbered `?` placeholders so the tag-name IN list can be
            // prepended without index conflicts.
            let mut trend_params: Vec<Box<dyn ToSql>> = Vec::new();
            let mut trend_clauses: Vec<String> = Vec::new();

            // Tag name filter (IN list)
            let name_phs: Vec<String> = top_tag_names
                .iter()
                .map(|name| {
                    trend_params.push(Box::new(name.clone()));
                    "?".to_string()
                })
                .collect();
            trend_clauses.push(format!("t.name IN ({})", name_phs.join(", ")));

            // Year range
            if filter.year_min.is_some() || filter.year_max.is_some() {
                let y_min = filter.year_min.unwrap_or(0);
                let y_max = filter.year_max.unwrap_or(9_999);
                trend_clauses.push(
                    "CAST(substr(i.created_at,1,4) AS INTEGER) BETWEEN ? AND ?".to_string(),
                );
                trend_params.push(Box::new(y_min));
                trend_params.push(Box::new(y_max));
            }

            // R18
            if let Some(ref val) = filter.r18 {
                match val.as_str() {
                    "hidden" => {
                        trend_clauses.push("i.x_restrict = ?".to_string());
                        trend_params.push(Box::new(0i64));
                    }
                    "only" => {
                        trend_clauses.push("i.x_restrict > ?".to_string());
                        trend_params.push(Box::new(0i64));
                    }
                    _ => {}
                }
            }

            // AI
            if let Some(ai) = filter.is_ai {
                trend_clauses.push("i.is_ai = ?".to_string());
                trend_params.push(Box::new(ai));
            }

            let trend_where = if trend_clauses.is_empty() {
                "1=1".to_string()
            } else {
                trend_clauses.join(" AND ")
            };

            let trend_sql = format!(
                "SELECT t.name as tag_name, t.translated_name, \
                        CAST(substr(i.created_at,1,4) AS INTEGER) as year, \
                        COUNT(DISTINCT i.id) as count \
                 FROM images i \
                 JOIN image_tags it ON i.id = it.image_id AND i.part = it.image_part \
                 JOIN tags t ON t.id = it.tag_id \
                 WHERE {} \
                 GROUP BY year, t.name ORDER BY year, count DESC",
                trend_where,
            );

            let mut stmt = conn
                .prepare(&trend_sql)
                .map_err(|e| format!("Tag trend prepare: {e}"))?;
            let refs: Vec<&dyn ToSql> = trend_params.iter().map(|b| b.as_ref()).collect();
            let rows = stmt
                .query_map(rusqlite::params_from_iter(&refs), |row| {
                    Ok(TagTrendItem {
                        year: row.get("year")?,
                        tag_name: row.get("tag_name")?,
                        translated_name: row.get("translated_name")?,
                        count: row.get("count")?,
                    })
                })
                .map_err(|e| format!("Tag trend query: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Tag trend collect: {e}"))?
        }
    };

    // ---- 16. Hidden Gems (bookmark/view ratio, part=0) ----
    let hidden_gems_sql = format!(
        "SELECT i.id, i.title, i.author_name, i.bookmark, i.view, \
                CAST(i.bookmark AS REAL) / MAX(i.view, 1) as ratio \
         FROM images i WHERE {} AND i.part = 0 \
         ORDER BY ratio DESC LIMIT 10",
        where_clause,
    );
    let hidden_gems: Vec<HiddenGem> = query_vec!(
        hidden_gems_sql,
        |row| {
            Ok(HiddenGem {
                id: row.get("id")?,
                title: row.get("title")?,
                author_name: row.get("author_name")?,
                bookmark: row.get("bookmark")?,
                view: row.get("view")?,
                ratio: row.get("ratio")?,
            })
        }
    );

    // ---- 17. Author Discovery ----
    let author_discovery_sql = format!(
        "SELECT i.author_id, i.author_name, i.author_account, \
                CAST(substr(MIN(i.created_at),1,4) AS INTEGER) as first_year, \
                COUNT(DISTINCT i.id) as works_count \
         FROM images i WHERE {} \
         GROUP BY i.author_id \
         HAVING COUNT(DISTINCT i.id) >= 10 \
         ORDER BY first_year DESC, works_count DESC",
        where_clause,
    );
    let author_discovery: Vec<AuthorDiscovery> = query_vec!(
        author_discovery_sql,
        |row| {
            Ok(AuthorDiscovery {
                author_id: row.get("author_id")?,
                author_name: row.get("author_name")?,
                author_account: row.get("author_account")?,
                first_year: row.get("first_year")?,
                works_count: row.get("works_count")?,
            })
        }
    );

    // ---- 18. Sanity Level Distribution ----
    let sanity_sql = format!(
        "SELECT i.sanity_level, COUNT(DISTINCT i.id) as count \
         FROM images i WHERE {} \
         GROUP BY i.sanity_level ORDER BY i.sanity_level",
        where_clause,
    );
    let sanity_levels: Vec<SanityLevelBucket> = query_vec!(
        sanity_sql,
        |row| {
            Ok(SanityLevelBucket {
                level: row.get::<_, i64>("sanity_level")? as i32,
                count: row.get("count")?,
            })
        }
    );

    // ---- Assemble result ----
    Ok(StatisticsResult {
        overview,
        author_ranking,
        tag_ranking,
        bookmark_distribution,
        view_distribution,
        yearly_trend,
        r18_ratio,
        ai_ratio,
        shape_distribution,
        top_bookmarked,
        top_viewed,
        author_works_distribution,
        r18_trend,
        ai_trend,
        tag_trend,
        hidden_gems,
        author_discovery,
        sanity_levels,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_where_empty() {
        let f = StatsFilter::default();
        let (sql, params) = f.build_where_clause();
        assert_eq!(sql, "1=1");
        assert!(params.is_empty());
    }

    #[test]
    fn test_build_where_year_range() {
        let f = StatsFilter {
            year_min: Some(2020),
            year_max: Some(2024),
            ..Default::default()
        };
        let (sql, params) = f.build_where_clause();
        assert!(sql.contains("BETWEEN ?1 AND ?2"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_build_where_r18_hidden() {
        let f = StatsFilter {
            r18: Some("hidden".to_string()),
            ..Default::default()
        };
        let (sql, params) = f.build_where_clause();
        assert!(sql.contains("i.x_restrict = ?1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_build_where_r18_only() {
        let f = StatsFilter {
            r18: Some("only".to_string()),
            ..Default::default()
        };
        let (sql, params) = f.build_where_clause();
        assert!(sql.contains("i.x_restrict > ?1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_build_where_r18_show() {
        let f = StatsFilter {
            r18: Some("show".to_string()),
            ..Default::default()
        };
        let (sql, params) = f.build_where_clause();
        assert_eq!(sql, "1=1");
        assert!(params.is_empty());
    }

    #[test]
    fn test_build_where_is_ai() {
        let f = StatsFilter {
            is_ai: Some(true),
            ..Default::default()
        };
        let (sql, params) = f.build_where_clause();
        assert!(sql.contains("i.is_ai = ?1"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_build_where_combined() {
        let f = StatsFilter {
            year_min: Some(2018),
            year_max: Some(2023),
            r18: Some("hidden".to_string()),
            is_ai: Some(false),
        };
        let (sql, params) = f.build_where_clause();
        assert!(sql.contains("BETWEEN ?1 AND ?2"));
        assert!(sql.contains("i.x_restrict = ?3"));
        assert!(sql.contains("i.is_ai = ?4"));
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn test_parse_bucket_bounds_closed() {
        assert_eq!(parse_bucket_bounds("1000-4999"), (1000, Some(4999)));
        assert_eq!(parse_bucket_bounds("0-99"), (0, Some(99)));
        assert_eq!(parse_bucket_bounds("50000-99999"), (50000, Some(99999)));
    }

    #[test]
    fn test_parse_bucket_bounds_open() {
        assert_eq!(parse_bucket_bounds("10000+"), (10000, None));
        assert_eq!(parse_bucket_bounds("100000+"), (100000, None));
    }
}
