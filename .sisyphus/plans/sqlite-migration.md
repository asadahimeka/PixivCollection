# SQLite Migration Plan

## TL;DR

> **Quick Summary**: Replace the current `JSON.parse(readTextFile(images.json))` → `window.__fullImages__` flow with a SQLite-backed Rust query layer. On first app startup (or when `images.json` changes), Rust ingests `images.json` into `images.db` via the `ensure_db` command. Tauri commands serve filtered/paginated/aggregated queries directly from SQLite, eliminating the 500MB+ JSON parse and 2-3GB heap overhead.
>
> **Deliverables**:
> - `src-tauri/src/db.rs` — SQLite connection management + schema + ingest + query builders
> - `src-tauri/src/main.rs` — new Tauri commands: `ensure_db`, `query_images`, `get_filter_options`, `import_json_to_db`
> - `web/src/store/index.ts` — async store actions calling Tauri commands instead of `window.__fullImages__`
> - `web/src/App.vue` — simplified init, remote mode removed, JSON import adapted
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 5 waves
> **Critical Path**: Rust db.rs → Rust ingest command → Rust query commands → Store refactor → App.vue cleanup

---

## Context

### Original Request
> "试验使用sqlite存储图片数据的方案，帮我列一份实施计划"

### Interview Summary
**Key Discussions**:
- Remove remote API mode entirely
- Keep `images.json` generation as archive only
- No full array in JS memory — store holds pagination window
- Filter logic moves from Pinia store to Rust SQL WHERE
- Shape filter (aspect ratio math) expressed as SQL expressions
- Search uses `LIKE '%term%'` to match current `.includes()` behavior
- Sidebar aggregates (years, authors, tags with counts) via separate Tauri command
- Connection management: `Mutex<Connection>` in Rust
- Schema versioning: rebuild = upgrade (Rust checks DB existence, ingests from images.json if absent)
- Tag storage: normalized schema with `tags(id, name, translated_name)` + `image_tags(...)` join table
- DB generation: Rust-side `ensure_db` command, NOT rename.mjs (avoids better-sqlite3 native module)
- Pagination: OFFSET/LIMIT with Rust handling multi-page boundary extension
- Import JSON to SQLite via `import_json_to_db` command (for `loadDataFromFile`)
- Comparison test script to verify filter parity

### Metis Review
**Identified Gaps** (addressed):
- **Multi-page pagination mismatch**: Flat-array OFFSET doesn't map cleanly to SQL. Resolved by having Rust return extra records beyond LIMIT when the boundary item belongs to a multi-page illustration.
- **Sidebar `getFilters()` scope**: Separate O(n) scan for filter option lists. Resolved by designing a dedicated `get_filter_options` Tauri command.
- **`dominant_color` dead field**: Identified as never populated. Excluded from SQLite schema.
- **`bookmark = -1` sentinel**: Special case for "unknown count." Explicitly handled in SQL WHERE builder.
- **Unicode search sensitivity**: SQLite LIKE is ASCII-only case-insensitive. Use `LOWER()` on both sides for Japanese/Unicode compatibility.
- **`loadDataFromFile()` / `exportFilteredData()`**: Two alternative data paths need migration. Import adapted to write to SQLite; export reads from store (unchanged).

---

## Work Objectives

### Core Objective
Migrate image metadata storage from JSON flat-file → SQLite query engine, eliminating UI-blocking JSON.parse and reducing peak heap memory from 2-3GB to <100MB.

### Concrete Deliverables
- Rust `ensure_db` command: reads `images.json`, normalizes tags, writes `images.db` with WAL mode + indexes
- `images.db` with normalized schema: `images` + `tags(id, name, translated_name)` + `image_tags(image_id, image_part, tag_id)`
- Tauri command `query_images` accepting filters + pagination, returning `{ images, total }`
- Tauri command `get_filter_options` returning sidebar aggregate data
- Tauri command `import_json_to_db` for file-import workflow
- Store actions migrated from in-memory array manipulation to async Tauri command calls
- App.vue init simplified: call `ensure_db` → `loadImagesByPage` → done

### Definition of Done
- [ ] `ensure_db` correctly ingests `images.json` into `images.db` (verify with sqlite3 CLI)
- [ ] `yarn dev` → app starts without loading `images.json`
- [ ] Gallery renders with correct images and pagination
- [ ] All 8 filter dimensions (R18, search, bookmark, year, author, tag, shape, size) produce identical results to current JS `imageFilter()`
- [ ] Sidebar shows correct years/authors/tags with counts
- [ ] `loadDataFromFile('images.json')` imports correctly into SQLite
- [ ] `exportFilteredData()` works unchanged
- [ ] Sort (id_desc, id_asc, bookmark_desc) matches current behavior

### Must Have
- Zero `JSON.parse(readTextFile(images.json))` calls on app startup
- `window.__fullImages__` no longer populated on init
- All filters match current JS behavior exactly
- Pagination never splits multi-page illustrations across page boundaries

### Must NOT Have (Guardrails)
- No FTS5 full-text search (stay with LIKE %substring%)
- No author normalization into separate table (keep denormalized)
- No ORM (hand-written rusqlite queries only)
- No schema migration tooling (rebuild DB = upgrade)
- No removal of `__fullImages__` global yet (keep for future cleanup)
- No reading `images.json` on app startup (still generated for archive only)

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO (no test framework)
- **Automated tests**: Comparison script only
- **Framework**: Standalone Node.js script that runs old JS filter vs new SQL query on same data
- **Comparison script**: Reads `images.json`, processes through old `imageFilter()` logic AND sends same filters to SQLite, asserts identical result sets

### QA Policy
Every task includes agent-executed QA scenarios. Evidence saved to `.sisyphus/evidence/task-{N}-{scenario}.{ext}`.

- **Frontend/UI**: Playwright — navigate gallery, toggle filters, assert DOM content
- **Backend/Rust**: Bash — run SQLite CLI to inspect schema and data, run Tauri commands via CLI if possible
- **Comparison**: Bash — run comparison script, diff outputs

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (parallel — Rust foundation):
├── Task 1: rename.mjs — keep JSON generation, remove DB writing
├── Task 2: Rust — Cargo.toml + db.rs (connection mgmt, schema, ensure_db)
├── Task 3: Rust — Query types + filter builder with normalized tags

Wave 2 (parallel — Tauri commands, depends on Wave 1):
├── Task 4: Rust — ensure_db command (calls db::ensure_db)
├── Task 5: Rust — query_images command (paginated filtered query)
├── Task 6: Rust — get_filter_options command (sidebar aggregates)
├── Task 7: Rust — import_json_to_db command (loadDataFromFile + dedup)

Wave 3 (parallel — frontend, depends on Wave 2):
├── Task 8: Store — loadImagesByPage → invoke('query_images')
├── Task 9: Store — sortImages → invoke('query_images')
├── Task 10: Store — counts → invoke('query_images' count-only)
├── Task 11: Store — remove loadFilteredImages + imageFilter
├── Task 12: Sidebar — getFilters → invoke('get_filter_options')
├── Task 13: Sidebar — loadDataFromFile → invoke('import_json_to_db')

Wave 4 (cleanup):
├── Task 14: App.vue — add ensure_db call, remove remote API mode, simplify init

Wave 5 (verification):
├── Task 15: Build comparison test script (JS filter vs SQL)

Critical Path: T2 → T4 → T5 → T8 → T14
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 4-5 (Waves 1-2)
```

### Dependency Matrix
- T1 (rename.mjs): none — Wave 1
- T2 (db.rs + schema + ensure_db): none — Wave 1
- T3 (types+filter builder): none — Wave 1
- T4 (ensure_db): T2 — Wave 2
- T5 (query_images): T2, T3 — Wave 2
- T6 (get_filter_options): T2, T3 — Wave 2
- T7 (import_json_to_db): T2 — Wave 2
- T8 (store pagination): T5 — Wave 3
- T9 (store sort): T5 — Wave 3
- T10 (store counts): T5 — Wave 3
- T11 (remove loadFilteredImages): T8 — Wave 3
- T12 (sidebar getFilters): T6 — Wave 3
- T13 (sidebar import): T7 — Wave 3
- T14 (App.vue cleanup): T2 (ensure_db), T8, T11, T12 — Wave 4
- T15 (comparison script): T2, T4 — Wave 5

### Tauri Command API Design

```
ensure_db(imgDir: string) → EnsureDbResult

EnsureDbResult {
  status: "created" | "exists" | "error",
  imported: number,       // images ingested (0 if already exists)
}

---

query_images(imgDir: string, query: ImageQuery) → QueryResult

ImageQuery {
  offset?: number,       // pagination offset (default 0)
  limit?: number,        // page size (default 30)
  sort_by?: string,      // "id_desc" | "id_asc" | "bookmark_desc" (default "id_desc")
  // Filters (all optional):
  search?: string,       // LIKE '%term%' across id, title, author, tags (via image_tags + tags JOIN)
  year?: number,         // 1 = "before 2000"
  tag?: string,          // exact tag name match (via image_tags + tags JOIN)
  author_id?: number,
  shape?: string,        // "horizontal" | "vertical" | "square" | "ratio-4:3" | etc.
  width_min?: number,
  width_max?: number,
  height_min?: number,
  height_max?: number,
  bookmark_min?: number,  // -1 = "unbookmarked only"
  r18?: string,          // "hidden" | "show" | "only"
  max_sanity_level?: number,
}

QueryResult {
  images: Image[],
  total: number,         // total matching images (for pagination UI)
}

---

get_filter_options(imgDir: string) → FilterOptions

FilterOptions {
  years: { year: number, count: number }[],
  authors: { id: number, name: string, account: string, count: number }[],
  tags: { name: string, translated_name: string | null, count: number }[],
}

---

import_json_to_db(imgDir: string, jsonContent: string) → { imported: number, skipped: number }
```

---

## TODOs

- [ ] 1. **rename.mjs — Keep JSON generation only (no DB writing)**

  **What to do**:
  - No changes needed to `rename.mjs` itself — it already generates `images.json`
  - Ensure it produces correct output (it already does)
  - The JSON is now consumed by Rust's `ensure_db` command instead of the frontend
  - Verify that `isAI` field is correctly populated (it already is, via `isAiIllust()` in rename.mjs)

  **Must NOT do**:
  - Don't add any SQLite logic here (DB generation moved to Rust)
  - Don't remove JSON generation (keep as source for Rust ingest)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (trivial — only verification)
  - **Blocked By**: None (already works)

  **Acceptance Criteria**:
  - [ ] `node scripts/rename.mjs` runs without errors
  - [ ] `{imgDir}/data/images.json` is generated as before

  **Commit**: NO (no changes needed to this file)

- [ ] 2. **Rust — Cargo.toml + db.rs (connection mgmt, schema, ingest)**

  **What to do**:
  - Add to `src-tauri/Cargo.toml`:
    ```toml
    rusqlite = { version = "0.31", features = ["bundled"] }
    serde = { version = "1.0", features = ["derive"] }
    serde_json = "1.0"
    ```
  - Create `src-tauri/src/db.rs` — this is the largest Rust module. It handles:

  **2a. Connection management**
  ```rust
  use std::sync::{Mutex, OnceLock};
  
  static DB: OnceLock<Mutex<Connection>> = OnceLock::new();
  
  pub fn init_db(img_dir: &str) -> Result<(), String> {
      let db_path = format!("{}/data/images.db", img_dir);
      let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
      conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
          .map_err(|e| e.to_string())?;
      DB.set(Mutex::new(conn)).map_err(|_| "DB already initialized".to_string())?;
      Ok(())
  }
  
  pub fn get_conn() -> Result<MutexGuard<'static, Connection>, String> {
      DB.get().ok_or("DB not initialized".to_string())?
         .lock().map_err(|e| e.to_string())
  }
  ```

  **2b. Schema constants** (normalized tags + size index)
  ```sql
  CREATE TABLE IF NOT EXISTS images (
      id INTEGER NOT NULL, part INTEGER NOT NULL, len INTEGER NOT NULL DEFAULT 1,
      title TEXT NOT NULL DEFAULT '', width INTEGER NOT NULL DEFAULT 0,
      height INTEGER NOT NULL DEFAULT 0, ext TEXT NOT NULL DEFAULT '',
      author_id INTEGER NOT NULL DEFAULT 0, author_name TEXT NOT NULL DEFAULT '',
      author_account TEXT NOT NULL DEFAULT '', bookmark INTEGER NOT NULL DEFAULT 0,
      view INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT '',
      sanity_level INTEGER NOT NULL DEFAULT 0, x_restrict INTEGER NOT NULL DEFAULT 0,
      is_ai INTEGER NOT NULL DEFAULT 0,
      img_s TEXT NOT NULL DEFAULT '', img_m TEXT NOT NULL DEFAULT '',
      img_l TEXT NOT NULL DEFAULT '', img_o TEXT NOT NULL DEFAULT '',
      PRIMARY KEY (id, part)
  );

  -- Normalized tags (document's approach — saves space, supports rename)
  CREATE TABLE IF NOT EXISTS tags (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL UNIQUE,
      translated_name TEXT
  );

  CREATE TABLE IF NOT EXISTS image_tags (
      image_id INTEGER NOT NULL, image_part INTEGER NOT NULL,
      tag_id INTEGER NOT NULL,
      PRIMARY KEY (image_id, image_part, tag_id),
      FOREIGN KEY (image_id, image_part) REFERENCES images(id, part),
      FOREIGN KEY (tag_id) REFERENCES tags(id)
  );

  CREATE TABLE IF NOT EXISTS _meta (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
  );
  ```

  Indexes (created AFTER bulk insert for performance):
  ```sql
  CREATE INDEX IF NOT EXISTS idx_images_author ON images(author_id);
  CREATE INDEX IF NOT EXISTS idx_images_bookmark ON images(bookmark);
  CREATE INDEX IF NOT EXISTS idx_images_restrict ON images(x_restrict);
  CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at);
  CREATE INDEX IF NOT EXISTS idx_images_size ON images(width, height);
  CREATE INDEX IF NOT EXISTS idx_images_is_ai ON images(is_ai);
  CREATE INDEX IF NOT EXISTS idx_image_tags_tag ON image_tags(tag_id);
  CREATE INDEX IF NOT EXISTS idx_image_tags_image ON image_tags(image_id, image_part);
  CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
  ```

  **2c. `ensure_db` logic** (ingest `images.json` → SQLite)
  ```rust
  pub fn ensure_db(img_dir: &str) -> Result<EnsureDbResult, String> {
      let db_path = format!("{}/data/images.db", img_dir);
      let json_path = format!("{}/data/images.json", img_dir);
      
      // If DB already exists, open it and set global connection
      if Path::new(&db_path).exists() {
          let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
          conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
              .map_err(|e| e.to_string())?;
          DB.set(Mutex::new(conn)).map_err(|_| "DB already initialized".to_string())?;
          return Ok(EnsureDbResult { status: "exists", imported: 0 });
      }
      
      // Read and parse JSON
      let json_str = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
      let entries: Vec<serde_json::Value> = serde_json::from_str(&json_str)
          .map_err(|e| format!("Failed to parse images.json: {}", e))?;
      
      // Create DB with WAL mode
      let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
      conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
          .map_err(|e| e.to_string())?;
      
      // Create tables first (no indexes — added after bulk insert)
      conn.execute_batch(CREATE_TABLES_SQL).map_err(|e| e.to_string())?;
      
      // Bulk insert in transaction
      let tx = conn.transaction().map_err(|e| e.to_string())?;
      let mut imported = 0i64;
      
      for entry in &entries {
          let id = entry["id"].as_i64().unwrap_or(0);
          let part = entry["part"].as_i64().unwrap_or(0);
          let tags_arr: Vec<serde_json::Value> = entry["tags"].as_array()
              .cloned().unwrap_or_default();
          
          // Insert image (skip if exists — idempotent)
          tx.execute(
              "INSERT OR IGNORE INTO images(id,part,len,title,width,height,ext,
               author_id,author_name,author_account,bookmark,view,created_at,
               sanity_level,x_restrict,is_ai,img_s,img_m,img_l,img_o)
               VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
              rusqlite::params![
                  id, part,
                  entry["len"].as_i64().unwrap_or(1),
                  entry["title"].as_str().unwrap_or(""),
                  entry["size"][0].as_i64().unwrap_or(0),
                  entry["size"][1].as_i64().unwrap_or(0),
                  entry["ext"].as_str().unwrap_or(""),
                  entry["author"]["id"].as_i64().unwrap_or(0),
                  entry["author"]["name"].as_str().unwrap_or(""),
                  entry["author"]["account"].as_str().unwrap_or(""),
                  entry["bookmark"].as_i64().unwrap_or(0),
                  entry["view"].as_i64().unwrap_or(0),
                  entry["created_at"].as_str().unwrap_or(""),
                  entry["sanity_level"].as_i64().unwrap_or(0),
                  entry["x_restrict"].as_i64().unwrap_or(0),
                  entry["is_ai"].as_bool().unwrap_or(false) as i64,
                  entry["images"]["s"].as_str().unwrap_or(""),
                  entry["images"]["m"].as_str().unwrap_or(""),
                  entry["images"]["l"].as_str().unwrap_or(""),
                  entry["images"]["o"].as_str().unwrap_or(""),
              ],
          ).map_err(|e| e.to_string())?;
          
          // Process tags
          for tag_val in &tags_arr {
              let name = tag_val["name"].as_str().unwrap_or("");
              let translated = tag_val["translated_name"].as_str();
              
              // Upsert tag (INSERT OR IGNORE avoids duplicate name)
              tx.execute(
                  "INSERT OR IGNORE INTO tags(name, translated_name) VALUES(?1,?2)",
                  rusqlite::params![name, translated],
              ).map_err(|e| e.to_string())?;
              
              // Get tag id
              let tag_id: i64 = tx.query_row(
                  "SELECT id FROM tags WHERE name = ?1",
                  rusqlite::params![name],
                  |row| row.get(0),
              ).map_err(|e| e.to_string())?;
              
              // Link image to tag
              tx.execute(
                  "INSERT OR IGNORE INTO image_tags(image_id, image_part, tag_id) VALUES(?1,?2,?3)",
                  rusqlite::params![id, part, tag_id],
              ).map_err(|e| e.to_string())?;
          }
          
          imported += 1;
      }
      
      tx.commit().map_err(|e| e.to_string())?;
      
      // Create indexes AFTER bulk insert (much faster)
      conn.execute_batch(CREATE_INDEXES_SQL).map_err(|e| e.to_string())?;
      
      // Record meta
      conn.execute(
          "INSERT OR REPLACE INTO _meta(key,value) VALUES('schema_version','1')",
          [],
      ).map_err(|e| e.to_string())?;
      
      // Set connection as global
      DB.set(Mutex::new(conn)).map_err(|_| "DB already initialized".to_string())?;
      
      Ok(EnsureDbResult { status: "created", imported })
  }
  ```

  **Must NOT do**:
  - Don't add r2d2 or connection pool (overkill)
  - Don't add Diesel or SeaORM
  - Don't normalize `authors` into separate table
  - Don't add FTS5 virtual tables

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: Tasks 4, 5, 6, 7

  **Acceptance Criteria**:
  - [ ] `cargo build` passes
  - [ ] `ensure_db` ingests a real `images.json` into a valid `images.db`
  - [ ] `sqlite3 {db} "SELECT COUNT(*) FROM images"` matches `jq length images.json`
  - [ ] `sqlite3 {db} "SELECT COUNT(*) FROM tags"` > 0
  - [ ] `sqlite3 {db} "SELECT COUNT(*) FROM image_tags"` > 0
  - [ ] Duplicate tags have the same `tags.id` (UNIQUE constraint on `name`)
  - [ ] Indexes exist and are used (verify with `EXPLAIN QUERY PLAN`)

  **QA Scenarios**:

  ```
  Scenario: Ingest and verify DB
    Tool: Bash
    Preconditions: images.json at known path
    Steps:
      1. Call ensure_db via cargo test or CLI
      2. sqlite3 "{imgDir}/data/images.db" "SELECT COUNT(*) FROM images;"
      3. jq length "{imgDir}/data/images.json"
    Expected Result: Both counts match
    Evidence: .sisyphus/evidence/task-2-ingest.txt

  Scenario: Verify normalized tags
    Tool: Bash
    Preconditions: DB with ingested data
    Steps:
      1. sqlite3 "{db}" "SELECT COUNT(*) FROM tags;"
      2. sqlite3 "{db}" "SELECT COUNT(*) FROM image_tags;"
    Expected Result: Both > 0, tags has unique names
    Evidence: .sisyphus/evidence/task-2-tags.txt

  Scenario: Idempotent re-ingest
    Tool: Bash
    Preconditions: DB exists
    Steps:
      1. Run ensure_db again
    Expected Result: Returns status="exists", imported=0 (no re-ingest)
    Evidence: .sisyphus/evidence/task-2-idempotent.txt
  ```

  **Commit**: YES
  - Message: `feat(src-tauri): add rusqlite with schema, ingest, and connection management`
  - Files: `src-tauri/Cargo.toml`, `src-tauri/src/db.rs`

- [ ] 3. **Rust — Define query types + SQL filter builder (normalized tags)**

  **What to do**:
  - Create `src-tauri/src/query.rs` with types and filter builder
  - Define serializable types (same as before, add `EnsureDbResult`):
    ```rust
    #[derive(Deserialize)]
    pub struct ImageQuery {
        pub offset: Option<i64>,
        pub limit: Option<i64>,
        pub sort_by: Option<String>,
        // ... all filter fields from API spec
    }

    #[derive(Serialize)]
    pub struct ImageRow {
        pub id: i64, pub part: i64, pub len: i64, pub title: String,
        pub width: i64, pub height: i64, pub ext: String,
        pub author_id: i64, pub author_name: String, pub author_account: String,
        pub bookmark: i64, pub view: i64, pub created_at: String,
        pub sanity_level: i64, pub x_restrict: i64, pub is_ai: bool,
        pub img_s: String, pub img_m: String, pub img_l: String, pub img_o: String,
    }

    #[derive(Serialize)]
    pub struct QueryResult {
        pub images: Vec<ImageRow>,
        pub total: i64,
    }

    #[derive(Serialize)]
    pub struct EnsureDbResult {
        pub status: String,  // "created" | "exists"
        pub imported: i64,
    }
    ```
  - Implement `ImageQuery::build_sql(&self) -> (String, Vec<Box<dyn ToSql>>)`:
    - Base: `SELECT i.* FROM images i WHERE 1=1`
    - Filter conditions (all AND-combined):
      - `r18 "hidden"` → `AND i.x_restrict < 1`
      - `r18 "only"` → `AND i.x_restrict >= 1`
      - `max_sanity_level` → `AND i.sanity_level <= ?`
      - **search** → `AND (CAST(i.id AS TEXT) LIKE ? OR i.title LIKE ? OR CAST(i.author_id AS TEXT) LIKE ? OR i.author_name LIKE ? OR EXISTS(SELECT 1 FROM image_tags it JOIN tags t ON it.tag_id = t.id WHERE it.image_id = i.id AND it.image_part = i.part AND (LOWER(t.name) LIKE LOWER(?) OR LOWER(t.translated_name) LIKE LOWER(?))))` — uses normalized tag schema with JOIN
      - `bookmark_min == -1` → `AND i.bookmark = -1`
      - `bookmark_min > 0` → `AND i.bookmark >= ?`
      - `year == 1` → `AND CAST(substr(i.created_at,1,4) AS INTEGER) < 2000`
      - `year > 1` → `AND CAST(substr(i.created_at,1,4) AS INTEGER) = ?`
      - **tag (exact match)** → `AND EXISTS(SELECT 1 FROM image_tags it JOIN tags t ON it.tag_id = t.id WHERE it.image_id = i.id AND it.image_part = i.part AND t.name = ?)`
      - `author_id` → `AND i.author_id = ?`
      - `shape` → translate to width/height comparison expression:
  - `"horizontal"` → `AND i.width > i.height * 1.2`
  - `"vertical"` → `AND i.height > i.width * 1.2`
  - `"square"` → `AND i.width BETWEEN i.height * 0.8 AND i.height * 1.2`
  - `"ratio-4:3"` → `AND ABS(i.width * 3 - i.height * 4) * 100 <= (i.width * 3 + i.height * 4) / 2`
  - `"ratio-16:9"` → `AND ABS(i.width * 9 - i.height * 16) * 100 <= (i.width * 9 + i.height * 16) / 2`
  - Thresholds must match the existing JS `imageFilter()` math exactly
      - `width_min/max`, `height_min/max` → range checks
    - ORDER BY: `i.id DESC, i.part ASC` (default) | `i.id ASC, i.part ASC` | `i.bookmark DESC, i.id DESC, i.part ASC`
    - Multi-page boundary handling: same approach (query LIMIT+OFFSET, extend if last row is incomplete multi-page)
    - Total count: `SELECT COUNT(*) FROM images i WHERE ...` (same WHERE, no ORDER)

  **Must NOT do**:
  - Don't use raw string interpolation (parameterized queries only)
  - Don't add FTS5

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: Tasks 4, 5, 6

  **Acceptance Criteria**:
  - [ ] Filter builder compiles without warnings
  - [ ] Search clause correctly joins through `image_tags` → `tags` for tag matching
  - [ ] Tag exact match correctly uses normalized schema
  - [ ] All filters produce parameterized SQL (no string interpolation)
  - [ ] Shape filter expression matches JS math exactly

  **QA Scenarios**:

  ```
  Scenario: Verify generated SQL for search
    Tool: Bash
    Preconditions: Query builder compiles
    Steps:
      1. cargo test -- --nocapture (print generated SQL)
    Expected Result: Search query includes JOIN through image_tags → tags
    Evidence: .sisyphus/evidence/task-3-search-sql.txt
  ```

  **Commit**: YES (groups with Task 2)
  - Message: `feat(src-tauri): add query types and SQL filter builder`
  - Files: `src-tauri/src/query.rs`

- [ ] 4. **Rust — ensure_db Tauri command**

  **What to do**:
  - In `src-tauri/src/main.rs` (or a new `commands.rs`), add:
    ```rust
    #[tauri::command]
    fn ensure_db(img_dir: String) -> Result<EnsureDbResult, String> {
        db::ensure_db(&img_dir)
    }
    ```
  - Register in `invoke_handler!`
  - The actual logic lives in `db.rs` (Task 2) — this command is a thin wrapper
  - Called from frontend `init()` before any `query_images` call

  **Must NOT do**:
  - Don't block the Tauri command with a long-running ingest without progress feedback (consider async or a future progress event)

  **Parallelization**:
  - **Can Run In Parallel**: Wave 2 (with Tasks 5, 6)
  - **Blocked By**: Task 2 (db.rs)

  **Acceptance Criteria**:
  - [ ] Command returns `{ status: "created", imported: N }` on first call
  - [ ] Command returns `{ status: "exists", imported: 0 }` on subsequent calls
  - [ ] Command returns error for invalid imgDir or corrupted JSON

  **QA Scenarios**:

  ```
  Scenario: First-run ingest
    Tool: Bash
    Preconditions: images.json exists, no images.db
    Steps:
      1. Invoke ensure_db with valid imgDir
    Expected Result: status="created", imported > 0
    Evidence: .sisyphus/evidence/task-4-first-run.txt

  Scenario: Idempotent second call — connection still valid
    Tool: Bash
    Preconditions: DB already exists
    Steps:
      1. Invoke ensure_db again
      2. Then call db::get_conn() (or invoke query_images with empty filter)
    Expected Result: status="exists", imported = 0; get_conn() returns Ok
    Evidence: .sisyphus/evidence/task-4-idempotent.txt
  ```

  **Commit**: YES (groups with Tasks 5, 6)
  - Message: `feat(src-tauri): add ensure_db, query_images, get_filter_options, import_json_to_db commands`
  - Files: `src-tauri/src/main.rs`

- [ ] 5. **Rust — query_images Tauri command**

  **What to do**:
  - In `src-tauri/src/main.rs` (or a new `commands.rs`), define:
    ```rust
    #[tauri::command]
    fn query_images(img_dir: String, query: ImageQuery) -> Result<QueryResult, String> {
        let conn = db::get_conn()?;
        // Build SQL, execute, map to ImageRow, handle multi-page boundary
        // Return QueryResult { images, total }
    }
    ```
  - Register in `invoke_handler!`
  - Handle errors gracefully: return `Err(String)` for corrupt DB, missing DB, etc.
  - Multi-page boundary logic:
    1. Execute main paginated query
    2. If result has rows and `last_row.part < last_row.len - 1`:
       - Query additional rows: `SELECT ... FROM images WHERE id = ? AND part > ? ORDER BY part ASC LIMIT ?`
       - Concat to result
    3. Return combined result + total count

  **Must NOT do**:
  - Don't unwrap() panics — propagate errors as Strings
  - Don't open a new connection per call (use the cached one from db.rs)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 6, 7)
  - **Blocked By**: Tasks 2, 3

  **Acceptance Criteria**:
  - [ ] Command returns correct images for basic query (no filters, default sort)
  - [ ] Command returns correct total count
  - [ ] Command handles all filter dimensions
  - [ ] Multi-page boundary: if last item has len > part+1, result includes continuation rows
  - [ ] Error: missing DB returns Err with descriptive message

  **QA Scenarios**:

  ```
  Scenario: Basic paginated query
    Tool: Bash
    Preconditions: App running with valid images.db
    Steps:
      1. Use Tauri CLI or inspect devtools: invoke('query_images', { imgDir: '...', query: { offset: 0, limit: 5, sort_by: 'id_desc' } })
    Expected Result: Returns 5 images, total matches COUNT(*)
    Evidence: .sisyphus/evidence/task-5-basic-query.json

  Scenario: Filter by year
    Tool: Bash (with Tauri command testing via cargo test or manual invoke)
    Preconditions: Same as above
    Steps:
      1. query with year=2024
    Expected Result: All returned images have created_at year 2024
    Evidence: .sisyphus/evidence/task-5-year-filter.json

  Scenario: Filter by search
    Tool: Bash
    Preconditions: Same
    Steps:
      1. query with search='初音'
    Expected Result: Results contain '初音' in title, author name, or tags
    Evidence: .sisyphus/evidence/task-5-search-filter.json
  ```

  **Commit**: YES
  - Message: `feat(src-tauri): add query_images Tauri command with filter builder`
  - Files: `src-tauri/src/main.rs`, `src-tauri/src/commands.rs` (if extracted)

- [ ] 6. **Rust — get_filter_options Tauri command**

  **What to do**:
  - Define return type:
    ```rust
    #[derive(Serialize)]
    pub struct FilterOptions {
        pub years: Vec<YearOption>,
        pub authors: Vec<AuthorOption>,
        pub tags: Vec<TagOption>,
    }
    ```
  - Implement 3 aggregate queries:
    1. **Years**: `SELECT CAST(substr(created_at,1,4) AS INTEGER) as year, COUNT(*) as count FROM images GROUP BY year ORDER BY year DESC`
    2. **Authors**: `SELECT author_id as id, author_name as name, author_account as account, COUNT(DISTINCT id) as count FROM images GROUP BY author_id ORDER BY count DESC LIMIT 100`
    3. **Tags**: `SELECT t.name, t.translated_name, COUNT(DISTINCT it.image_id) as count FROM tags t JOIN image_tags it ON it.tag_id = t.id GROUP BY t.name ORDER BY count DESC LIMIT 200`
  - Register as `get_filter_options` Tauri command

  **Must NOT do**:
  - Don't add pagination to aggregate results (return top N)
  - Don't filter the aggregates by current image filters (they show ALL possibilities)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5, 7)
  - **Blocked By**: Tasks 2, 3

  **Acceptance Criteria**:
  - [ ] Returns correct year list with counts
  - [ ] Returns correct author list with counts (unique illust count per author)
  - [ ] Returns correct tag list with counts (unique illust count per tag)
  - [ ] Results ordered appropriately (years desc, authors by count desc, tags by count desc)

  **QA Scenarios**:

  ```
  Scenario: Verify filter options
    Tool: Bash
    Preconditions: App running
    Steps:
      1. invoke('get_filter_options', { imgDir: '...' })
    Expected Result: Years, authors, tags arrays populated with correct counts
    Evidence: .sisyphus/evidence/task-6-filter-options.json
  ```

  **Commit**: YES (groups with Task 4)
  - Message: `feat(src-tauri): add get_filter_options command for sidebar aggregates`
  - Files: `src-tauri/src/main.rs`

- [ ] 7. **Rust — import_json_to_db Tauri command**

  **What to do**:
  - Define command:
    ```rust
    #[tauri::command]
    fn import_json_to_db(img_dir: String, json_content: String) -> Result<ImportResult, String>
    ```
  - Parse `json_content` as `Vec<serde_json::Value>` (flexible format)
  - Open DB at `{img_dir}/data/images.db`
  - Enable WAL mode
  - BEGIN transaction
  - For each entry: `INSERT OR IGNORE INTO images ...` (dedup by (id,part) PK)
  - Also insert tags
  - COMMIT
  - Return `{ imported: N, skipped: M }`
  - Handle malformed JSON gracefully

  **Must NOT do**:
  - Don't parse using `serde`'s derive struct (use `serde_json::Value` for flexibility)
  - Don't throw entire error stack to frontend (sanitize)

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5)
  - **Blocked By**: Task 2

  **Acceptance Criteria**:
  - [ ] Import valid JSON: all entries stored in DB
  - [ ] Import with duplicates: existing rows kept, new rows inserted
  - [ ] Import invalid JSON: returns error with message
  - [ ] Import empty array: returns imported=0

  **QA Scenarios**:

  ```
  Scenario: Import valid JSON
    Tool: Bash
    Preconditions: App running, temp JSON file with 5 entries
    Steps:
      1. invoke('import_json_to_db', { imgDir: '...', jsonContent: '[{...}, {...}]' })
    Expected Result: { imported: 5, skipped: 0 }
    Evidence: .sisyphus/evidence/task-7-import.txt

  Scenario: Import with duplicates
    Tool: Bash
    Preconditions: Same
    Steps:
      1. Repeat the same import
    Expected Result: { imported: 0, skipped: 5 }
    Evidence: .sisyphus/evidence/task-7-dedup.txt
  ```

  **Commit**: YES
  - Message: `feat(src-tauri): add import_json_to_db command for file import flow`
  - Files: `src-tauri/src/main.rs`

- [ ] 8. **Store — Rewrite loadImagesByPage to call query_images**

  **What to do**:
  - Add at top of `web/src/store/index.ts`:
    ```ts
    import { invoke } from '@tauri-apps/api/tauri'
    ```
  - Replace the current `loadImagesByPage` implementation
  - New flow:
    1. `store.curPageCursor` becomes a simple numeric offset (not a cursor into `__filteredImages__`)
    2. `loadImagesByPage(isFirstLoad)` calls:
       ```ts
       const result = await invoke<QueryResult>('query_images', {
         imgDir: window.__CONFIG__.imgDir,
         query: {
           offset: this.curPageCursor,
           limit: 60, // slightly more than 30 to handle multi-page extension
           sort_by: this.masonryConfig.imageSortBy,
           ...this.filterConfig, // map filter config to query params
         }
       })
       ```
    3. If `isFirstLoad`: replace `this.imagesFiltered` with `result.images`
    4. If appending (`fetchMore`): `this.imagesFiltered = this.imagesFiltered.concat(result.images)`
    5. Update `this.curPageCursor += result.images.length`
    6. `this.loadEnd = result.images.length < 30` (no more data)
  - Map `filterConfig` store state to `ImageQuery` shape before invoking
  - Filter mapping helper:
    ```ts
    function buildFilterQuery(filterConfig): ImageQuery {
      return {
        search: filterConfig.search.enable ? filterConfig.search.value : undefined,
        year: filterConfig.year.enable ? filterConfig.year.value : undefined,
        tag: filterConfig.tag.enable ? filterConfig.tag.name : undefined,
        author_id: filterConfig.author.enable ? filterConfig.author.id : undefined,
        shape: filterConfig.shape.enable ? filterConfig.shape.value : undefined,
        width_min: filterConfig.size.enable ? filterConfig.size.width.min : undefined,
        width_max: filterConfig.size.enable ? filterConfig.size.width.max : undefined,
        height_min: filterConfig.size.enable ? filterConfig.size.height.min : undefined,
        height_max: filterConfig.size.enable ? filterConfig.size.height.max : undefined,
        bookmark_min: filterConfig.bookmark.enable ? filterConfig.bookmark.min : undefined,
        r18: filterConfig.restrict.r18,
        max_sanity_level: filterConfig.restrict.maxSanityLevel,
        offset: 0,
        limit: 60,
        sort_by: this.masonryConfig.imageSortBy,
      }
    }
    ```

  **Must NOT do**:
  - Don't keep the old `w.__fullImages__` reference for this path
  - Don't keep `loadFilteredImages()` full-scan method (it will be removed in Task 10)

  **Parallelization**:
  - **Can Run In Parallel**: NO (one of the core store rewrites)
  - **Parallel Group**: Wave 3 (but sequential within — do Task 8 first, then 9, 10, 11)
  - **Blocked By**: Task 5 (query_images command)

  **Note**: Since all store tasks (8-11) modify `store/index.ts`, they should be done in sequence or with careful conflict management. Task 8 is the foundational change; others modify additional actions on the same file.

  **Acceptance Criteria**:
  - [ ] First page loads (offset=0, limit=60) with correct images
  - [ ] "Load more" appends next batch correctly
  - [ ] curPageCursor advances correctly accounting for multi-page extension
  - [ ] loadEnd triggers when no more images
  - [ ] Filters are correctly mapped and sent to Tauri command

  **QA Scenarios**:

  ```
  Scenario: First page loads via SQLite
    Tool: Playwright (browser automation)
    Preconditions: App running, valid images.db exists, no __fullImages__ populated
    Steps:
      1. Navigate to app
      2. Wait for gallery to render
      3. Check that image elements exist in DOM
    Expected Result: Gallery shows first batch of images (not empty, no loading error)
    Evidence: .sisyphus/evidence/task-8-first-page.png

  Scenario: Load more pagination
    Tool: Playwright
    Preconditions: Gallery showing first batch
    Steps:
      1. Click "加载更多" button (or scroll to trigger)
      2. Wait for new images to appear
    Expected Result: More images appended, total count increases
    Evidence: .sisyphus/evidence/task-8-load-more.png
  ```

  **Commit**: YES (groups with Tasks 8, 9, 10)
  - Message: `refactor(web): migrate store pagination to SQLite-backed Tauri commands`
  - Files: `web/src/store/index.ts`

- [ ] 9. **Store — Rewrite sortImages to call query_images**

  **What to do**:
  - Replace current `sortImages()` action:
    - Remove the old `w.__fullImages__.sort(...)` in-place sort
    - New flow:
      ```ts
      async sortImages() {
        this.curPageCursor = 0
        this.loadEnd = false
        this.imagesFiltered = []
        await this.loadImagesByPage(true) // uses current sort_by from masonryConfig
      }
      ```
    - The `masonryConfig.imageSortBy` is already sent to `query_images` in Task 7's filter builder
    - The sort effectively happens via SQL `ORDER BY`

  **Must NOT do**:
  - Don't mutate `w.__fullImages__` in-place (it's no longer the data source)

  **Parallelization**:
  - **Can Run In Parallel**: NO (same file as Task 8)
  - **Blocked By**: Task 8

  **Acceptance Criteria**:
  - [ ] Sort by id_desc returns most recent first
  - [ ] Sort by id_asc returns oldest first
  - [ ] Sort by bookmark_desc returns most bookmarked first
  - [ ] Changing sort reloads the page from offset 0
  - [ ] Sorting with active filters works correctly

  **QA Scenarios**:

  ```
  Scenario: Change sort order
    Tool: Playwright
    Preconditions: Gallery loaded
    Steps:
      1. Open sidebar
      2. Change "排序" dropdown to "收藏数"
      3. Wait for reload
    Expected Result: Gallery shows images sorted by bookmark count
    Evidence: .sisyphus/evidence/task-9-sort.png
  ```

  **Commit**: YES (groups with Tasks 8, 9, 10)

- [ ] 10. **Store — Rewrite counts to use query_images count**

  **What to do**:
  - Currently `updateFullCounts()` and `updateFilteredCounts()` iterate `__fullImages__` with Set/map/flatMap
  - Replace both with a single async action:
    ```ts
    async updateCounts() {
      const result = await invoke<QueryResult>('query_images', {
        imgDir: window.__CONFIG__.imgDir,
        query: {
          limit: 0, // tell Rust to only return count, not images
          ...buildFilterQuery(this.filterConfig),
        }
      })
      // But total isn't enough — we also need illustCount, authorCount, tagCount
      // Options:
      // A) Have Rust return all 4 counts (preferred — add fields to QueryResult)
      // B) Call query_images + 3 separate count queries
    }
    ```
  - **Recommendation**: Extend the `query_images` Tauri command to optionally return `distinct_illust_count`, `distinct_author_count`, `distinct_tag_count` when `include_counts: true` is set in the query. This avoids multiple round-trips.
  - Remove `updateFullCounts` and `updateFilteredCounts` actions from store (they reference `window.__fullImages__`)
  - Update `filteredCounts` reactive state from the new async method
  - Call after: initial load, filter change, sort change

  **Must NOT do**:
  - Don't iterate `__fullImages__` or `__filteredImages__` for counts
  - Don't make 4 separate Tauri calls for counts

  **Parallelization**:
  - **Can Run In Parallel**: NO (same file as Tasks 8, 9)
  - **Blocked By**: Task 8

  **Acceptance Criteria**:
  - [ ] `total` matches `COUNT(*)` in DB
  - [ ] `illustCount` matches `COUNT(DISTINCT id)` (unique illustrations)
  - [ ] `authorCount` matches `COUNT(DISTINCT author_id)` (unique authors)
  - [ ] `tagCount` matches `COUNT(DISTINCT tag.name JOIN images...)` (unique tags across filtered images)
  - [ ] Counts update correctly when filters change

  **QA Scenarios**:

  ```
  Scenario: Counts display correctly
    Tool: Playwright
    Preconditions: Gallery loaded
    Steps:
      1. Check sidebar for "作品 N 件 / 画师 M 人 / 标签 K 个"
      2. Toggle a filter (e.g., R18 hidden → show)
    Expected Result: Counts update to reflect filtered result
    Evidence: .sisyphus/evidence/task-10-counts.png
  ```

  **Commit**: YES (groups with Tasks 8, 9, 10)

- [ ] 11. **Store — Remove loadFilteredImages full-scan action**

  **What to do**:
  - The `loadFilteredImages()` action iterates ALL `__fullImages__` entries applying `imageFilter()`:
    ```ts
    loadFilteredImages() {
      const res: Image[] = []
      for (let i = 0; i < w.__fullImages__.length; i++) {
        if (this.imageFilter(w.__fullImages__[i])) res.push(w.__fullImages__[i])
      }
      this.imagesFiltered = res
    }
    ```
  - This is no longer needed — all filtering happens in SQL via `query_images`
  - **Action**: Remove the `loadFilteredImages` action entirely
  - Any caller (currently `watch` on `filterConfig` in App.vue) should be updated to call `loadImagesByPage(true)` instead
  - Remove the `imageFilter` getter — it was the JS filtering logic that's now in Rust SQL

  **Must NOT do**:
  - Don't keep `imageFilter` as dead code (remove it to avoid confusion)
  - Don't forget to update the `watch` in App.vue that triggered `loadFilteredImages`

  **Parallelization**:
  - **Can Run In Parallel**: NO (same file as 8, 9, 10)
  - **Blocked By**: Task 8

  **Acceptance Criteria**:
  - [ ] `loadFilteredImages()` no longer exists in store
  - [ ] `imageFilter` getter no longer exists in store
  - [ ] All callers updated to use `loadImagesByPage(true)`
  - [ ] Filter changes still trigger gallery update

  **Commit**: YES (groups with Tasks 8, 9, 10)

- [ ] 12. **Sidebar — Rewrite getFilters to call get_filter_options**

  **What to do**:
  - In `web/src/components/Sidebar/Sidebar.vue`, find the `getFilters()` method
  - Current implementation iterates `__fullImages__` to build:
    - Year list: `new Set(__fullImages__.map(i => i.created_at.split('-')[0]))`
    - Author list with counts: `{ id, name, account, count }`
    - Tag list with counts: `{ name, translated_name, count }`
  - Replace with async call:
    ```ts
    async function getFilters() {
      const result = await invoke<FilterOptions>('get_filter_options', {
        imgDir: window.__CONFIG__.imgDir,
      })
      this.yearList = result.years
      this.authorList = result.authors
      this.tagList = result.tags
    }
    ```
  - Call `getFilters()` after initial data load (same trigger as before)
  - The `FilterOptions` Rust response type may need a corresponding TS type:
    ```ts
    interface FilterOptions {
      years: { year: number; count: number }[]
      authors: { id: number; name: string; account: string; count: number }[]
      tags: { name: string; translated_name: string | null; count: number }[]
    }
    ```

  **Must NOT do**:
  - Don't keep the old `__fullImages__` iteration as a fallback
  - Don't add the call inside the store (keep it in the component, it's UI state)

  **Parallelization**:
  - **Can Run In Parallel**: YES (sidebar is independent of store refactor)
  - **Parallel Group**: Wave 3 (with Tasks 8-11, 13)
  - **Blocked By**: Task 6 (get_filter_options command)

  **Acceptance Criteria**:
  - [ ] Year filter dropdown has correct years with counts
  - [ ] Author filter dropdown has correct authors with counts
  - [ ] Tag filter dropdown has correct tags with counts
  - [ ] All lists ordered correctly (years desc, authors/tags by count desc)
  - [ ] Performance: call completes in <1s for 500k images

  **QA Scenarios**:

  ```
  Scenario: Sidebar filter options match DB
    Tool: Playwright
    Preconditions: App loaded with data
    Steps:
      1. Open sidebar
      2. Check year dropdown options
      3. Check author list
      4. Check tag list
    Expected Result: Options populated and match expected distribution
    Evidence: .sisyphus/evidence/task-12-sidebar-filters.png
  ```

  **Commit**: YES
  - Message: `refactor(web): migrate sidebar filter options to SQLite query`
  - Files: `web/src/components/Sidebar/Sidebar.vue`

- [ ] 13. **Sidebar — Rewrite loadDataFromFile to use import_json_to_db**

  **What to do**:
  - In `web/src/components/Sidebar/Sidebar.vue`, find `loadDataFromFile()` method
  - Current implementation reads a JSON file via Tauri dialog and parses it client-side
  - New flow:
    1. Keep the file picker dialog (unchanged)
    2. Read file content as string (via Tauri `readTextFile`)
    3. Call `invoke('import_json_to_db', { imgDir, jsonContent })` instead of `JSON.parse`
    4. Show import result (imported/skipped count) to user
    5. Reload gallery and sidebar filters after import
  - Remove the client-side dedup logic (SQLite's INSERT OR IGNORE handles it)
  - Update UI to show "已导入 N 条，跳过 M 条重复"

  **Must NOT do**:
  - Don't `JSON.parse` in the frontend (let Rust handle it)
  - Don't modify `window.__fullImages__` after import

  **Parallelization**:
  - **Can Run In Parallel**: YES (sidebar file is separate)
  - **Parallel Group**: Wave 3 (with Tasks 8-12)
  - **Blocked By**: Task 7 (import_json_to_db command)

  **Acceptance Criteria**:
  - [ ] File picker dialog opens
  - [ ] Selected JSON file is imported to SQLite
  - [ ] Import count shown to user
  - [ ] Duplicate entries are skipped
  - [ ] Gallery refreshes after import

  **QA Scenarios**:

  ```
  Scenario: Import JSON file via dialog
    Tool: Playwright
    Preconditions: App running, sample images.json exists
    Steps:
      1. Click "导入数据" button in sidebar
      2. Select JSON file in file dialog
      3. Wait for import completion message
    Expected Result: "已导入 N 条，跳过 M 条" message shown
    Evidence: .sisyphus/evidence/task-13-import-result.png
  ```

  **Commit**: YES
  - Message: `refactor(web): migrate loadDataFromFile to use import_json_to_db`
  - Files: `web/src/components/Sidebar/Sidebar.vue`

- [ ] 14. **App.vue — Remove remote API mode + simplify init**

  **What to do**:
  - Remove the entire remote API code path in `web/src/App.vue`:
    - Remove `fetchUserBookmarks()` function
    - Remove `transformResData()` function
    - Remove the `if (__CONFIG__.userId)` branch from `init()` and `fetchMore()`
    - Remove `__CONFIG__.userId` reference from UI template
    - Remove userId input UI elements
    - Remove `userId` ref variable
    - Remove `__PXCT_USER_ID` localStorage persistence in `saveReload()`
  - Simplify `init()`:
    - Remove `readTextFile(__CONFIG__.jsonPath)` / `fetch(...images.json)` code paths
    - Remove `JSON.parse(contents)` call
    - Remove `window.__fullImages__ = contents` assignment
    - Replace with:
      ```ts
      async function init() {
        try {
          loading.value = true
          store.curPageCursor = 0
          store.loadEnd = false
          store.imagesFiltered = []
          if (isTauri && __CONFIG__.imgDir) {
            await invoke('ensure_db', { imgDir: __CONFIG__.imgDir })
            await store.loadImagesByPage(true)
            // getFilters will be called by Sidebar on mount
          }
        } catch (e) {
          console.error(e)
        } finally {
          loading.value = false
        }
      }
      ```
    - Remove `store.updateFullCounts()` call (replaced by Task 9)
  - Update the `watch` on `filterConfig` in `init()`:
    - Change from calling `store.loadFilteredImages()` to `store.loadImagesByPage(true)` (re-query with new filters)

  **Must NOT do**:
  - Don't keep any reference to `__fullImages__` in App.vue
  - Don't remove `isTauri` check (important for web-only testing)
  - Don't break `exportFilteredData()` (reads from store, independent of data source)

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Wave**: 4 (sequential — must come after store refactor is stable)
  - **Blocked By**: Tasks 8, 11 (store refactored), Task 12 (sidebar works)

  **Acceptance Criteria**:
  - [ ] App loads with only `imgDir` configured (no `jsonPath`, no `userId`)
  - [ ] Gallery loads from SQLite only
  - [ ] No `JSON.parse` or `readTextFile` of images.json during init
  - [ ] Filter change triggers `loadImagesByPage(true)` not `loadFilteredImages()`
  - [ ] No reference to `fetchUserBookmarks` or `transformResData`
  - [ ] userId input UI no longer visible

  **QA Scenarios**:

  ```
  Scenario: App loads without images.json
    Tool: Playwright
    Preconditions: DB exists, but images.json is deleted/empty
    Steps:
      1. Launch app
      2. Wait for gallery to render
    Expected Result: Gallery loads correctly from SQLite
    Evidence: .sisyphus/evidence/task-14-sqlite-only-load.png

  Scenario: Remove remote mode — no userId UI
    Tool: Playwright
    Preconditions: App running
    Steps:
      1. Check page for "用户 ID" input
    Expected Result: No "用户 ID" input visible
    Evidence: .sisyphus/evidence/task-14-no-userid.png
  ```

  **Commit**: YES
  - Message: `refactor(web): remove remote API mode, simplify init to SQLite-only`
  - Files: `web/src/App.vue`

- [ ] 15. **Build filter parity comparison test script**

  **What to do**:
  - Create `scripts/comparison-test.mjs` at project root
  - This script:
    1. Reads `images.json` (full dataset)
    2. Opens `images.db`
    3. For each filter combination:
       a. Runs the old JS `imageFilter()` logic on the JSON array
       b. Runs the equivalent SQL query on SQLite
       c. Compares `result.length` and the first 10 IDs
    4. Reports: PASS/FAIL per filter combination
  - Filter combinations to test:
    - No filters (all images)
    - R18 hidden
    - R18 only
    - Search by tag name
    - Search by title word
    - Search by author name
    - Search by illust ID
    - Author filter
    - Tag filter (exact match)
    - Year filter (specific year)
    - Year filter (before 2000, value=1)
    - Bookmark min
    - Bookmark = -1 (unbookmarked)
    - Shape = square
    - Shape = horizontal
    - Shape = 4:3 ratio
    - Size min/max width
    - Size min/max height
    - Combined: year + author + bookmark
    - Combined: search + R18 + shape
  - Write results to stdout and optionally to a JSON report file

  **Must NOT do**:
  - Don't make this a full test suite (no test framework, no assertions that crash)
  - Don't modify any app files

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Wave**: 5 (verification)
  - **Blocked By**: Task 5 (query_images command), 1 (rename.mjs JSON gen)

  **Note**: This task can run in Wave 2/3 timeline once the Rust query builder is reasonably complete. It's standalone and doesn't touch the frontend.

  **Acceptance Criteria**:
  - [ ] Script runs without error
  - [ ] Reports PASS for all filter combinations
  - [ ] Can be run standalone: `node scripts/comparison-test.mjs`
  - [ ] Output includes: filter name, JS count, SQL count, matched/mismatched
  - [ ] Exit code 0 if all pass, non-zero if any mismatch

  **QA Scenarios**:

  ```
  Scenario: Run comparison test
    Tool: Bash
    Preconditions: images.json and images.db exist for same dataset
    Steps:
      1. Run: node scripts/comparison-test.mjs
    Expected Result: All filters PASS, exit code 0
    Evidence: .sisyphus/evidence/task-15-comparison.txt
  ```

  **Commit**: YES
  - Message: `test: add filter parity comparison script for JS vs SQL validation`
  - Files: `scripts/comparison-test.mjs`

---

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  - Read the plan end-to-end. Verify: all "Must Have" items are implemented, all "Must NOT Have" are absent. Check:
    - `ensure_db` command exists and runs before any query
    - `query_images` command exists with all filters (using normalized tag JOIN)
    - `get_filter_options` command exists
    - `import_json_to_db` command exists
    - Store no longer references `__fullImages__` for main data path
    - App.vue calls `ensure_db` on init, no longer reads `images.json`
    - No `JSON.parse` of images.json on startup
    - Remote API code removed
  - Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  - Run `cd web && npx vue-tsc --noEmit` (typecheck)
  - Run `cd web && yarn lint` (ESLint)
  - Run `cargo build` in src-tauri
  - Check Rust code for: unwrap() in production code, hardcoded paths, SQL injection risks (parameterized queries), error handling
  - Check JS/TS for: unused imports, `any` casts, console.log in production
  - Output: `Typecheck [PASS/FAIL] | Lint [PASS/FAIL] | Build [PASS/FAIL] | Rust issues [N] | JS issues [N] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high` (+ `playwright` skill if UI)
  - Start from clean state (no DB, no images.json configured)
  - Set up pxder → run rename.mjs to generate images.json
  - Launch app → ensure_db should create DB → verify gallery renders
  - Toggle each filter dimension individually — results match expectations
  - Toggle sort → verify order
  - Load more → verify pagination
  - Import a JSON file via loadDataFromFile (calls import_json_to_db)
  - Check sidebar filter options match data
  - Save evidence to `.sisyphus/evidence/final-qa/`
  - Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  - For each task: read "What to do" → read actual diff (git log/diff)
  - Verify 1:1 — everything in spec was built, nothing beyond spec was built
  - Check "Must NOT do" compliance
  - Detect cross-task contamination (Task N touching Task M's files)
  - Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Commit | Files | Message |
|--------|-------|---------|
| 1 | `src-tauri/Cargo.toml`, `src-tauri/src/db.rs`, `src-tauri/src/query.rs` | `feat(src-tauri): add rusqlite with schema, ingest, and filter builder` |
| 2 | `src-tauri/src/main.rs` | `feat(src-tauri): add ensure_db, query_images, get_filter_options, import_json_to_db commands` |
| 3 | `web/src/store/index.ts` | `refactor(web): migrate store to SQLite-backed async queries` |
| 4 | `web/src/components/Sidebar/Sidebar.vue` | `refactor(web): migrate sidebar filters and import to Tauri commands` |
| 5 | `web/src/App.vue` | `refactor(web): remove remote API mode, add ensure_db call, simplify init` |
| 6 | `scripts/comparison-test.mjs` | `test: add filter parity comparison script` |

---

## Success Criteria

### Verification Commands
```bash
# 1. Verify ingest
sqlite3 "$IMG_DIR/data/images.db" "SELECT COUNT(*) FROM images;"
jq length "$IMG_DIR/data/images.json"  # Should match

# 2. Verify normalized tags
sqlite3 "$IMG_DIR/data/images.db" "SELECT COUNT(DISTINCT t.name) FROM tags t;"
sqlite3 "$IMG_DIR/data/images.db" "SELECT COUNT(*) FROM image_tags;"

# 3. Verify no JSON read on startup
# Launch app, check DevTools Network tab for NO requests to images.json

# 4. Verify Rust commands work
cd src-tauri && cargo build

# 5. Verify filter parity
node scripts/comparison-test.mjs  # Should output: ALL FILTERS MATCH

# 6. Verify typecheck + lint
cd web && npx vue-tsc --noEmit && yarn lint
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] `ensure_db` creates DB from `images.json` on first run
- [ ] Comparison script passes (all filters match)
- [ ] Gallery loads without `images.json`
- [ ] All filters work end-to-end (via normalized tag joins)
- [ ] Sidebar aggregate counts match DB
- [ ] Import/export flows work
