# SQLite 性能优化 V2 — 快慢分离 + 增量更新

## TL;DR

> **Quick Summary**: 解决当前 SQLite 方案在 20 万作品/300MB 数据库下反而不如纯前端方案的性能问题。核心策略是"快慢分离"——`query_images` 只取图片列表（快车道），`COUNT(DISTINCT)` 聚合查询全部移到后台 `query_image_counts`（慢车道不阻塞 UI）。同时修复缺失索引、优化年份筛选利用现有索引、缓存侧边栏聚合数据、实现收藏更新后即时增量导入。
>
> **Deliverables**:
> - `query.rs` — 移除 `include_counts`，年份筛选改为可索引的范围查询，新增 `CountsResult` 结构体
> - `main.rs` — 简化 `query_images`（仅返回图片+标签），新增 `query_image_counts`、`get_full_counts`、`reimport_db`、`refresh_caches` 命令
> - `db.rs` — 新增 `idx_images_sanity_level` 索引，`ensure_db` 末尾加 `ANALYZE`，新增 `refresh_caches()` 函数；**双连接（Reader/Writer）解除 Mutex 瓶颈**
> - `store/index.ts` — 移除 `include_counts`，新增后台 counts 获取逻辑
> - `App.vue` — `init()` 精简为单次查询+缓存读取，`updateBookmark` 流程改为触发 `reimport_db` + 不 reload
> - `Sidebar.vue` — 适配缓存化 `get_filter_options`（微调）
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES — 3 waves
> **Critical Path**: Rust 查询优化 → 前端适配 → 验证

---

## Context

### Original Request
> 排查启动慢/筛选慢的根因，出优化方案，要求至少不比纯前端方案慢。考虑收藏更新后即时更新 DB。

### Interview Summary
**关键讨论**：
- 痛点按优先级：启动加载 = 筛选切换 = 搜索 = 侧边栏全部慢
- 侧边栏 counts（图片/作品/作者/标签）必须准确，但可以不实时，后台查即可，不阻塞图片列表
- 收藏更新后需要即时增量导入 DB，不改 pxder pipeline
- 搜索结果(LIKE)可以不优化，当前方案够用
- 后台 counts 方案：独立 Tauri 命令（方案 A）
- 侧边栏缓存：存 `_meta` 表
- 增量导入：Rust 端直接读 JSON 文件（不通过 IPC 传大 JSON）

**根因分析**：
```
问题 1（最严重）：query_images 每次调用做 3-5 条 SQL，包括 COUNT(DISTINCT) 三重 JOIN
   → 启动时调用 2 次（共计 6-10 条 SQL）
   → 每次筛选又调 1 次（3-5 条 SQL）

问题 2：年份筛选 CAST(substr(created_at,1,4)) 无法使用 idx_images_created_at 索引
   → 每年份筛选都全表扫描 20 万行

问题 3：缺少 idx_images_sanity_level 索引
   → 默认所有查询都带 sanity_level <= 6，但没有索引

问题 4：get_filter_options 每次做 3 个聚合全表扫描
   → 涉及 tags + image_tags 的三重 JOIN，百万级行扫描

问题 5：收藏更新后 location.reload() 导致全量重新导入
   → 浪费已有数据，也不利于增量更新
```

### Metis Review
**Gaps identified** (auto-resolved in plan below):

**CRITICAL — Migration path for existing DBs**:
`create_indexes()` is only called on the import path. Existing DB users hitting the "exists" path in `ensure_db` (line 258) will **never** get `idx_images_sanity_level` or `ANALYZE`. Resolution: Add `CREATE INDEX IF NOT EXISTS` + `ANALYZE` to the "exists" path — idempotent, safe on every startup.

**Minor — Cache fallback**:
Corrupted `_meta` cache entries (disk error, interrupted write) would crash `get_filter_options`. Resolution: Add JSON parse error handling with fallback to full-scan queries.

**Minor — Atomic cache writes**:
`refresh_caches` writes multiple `_meta` keys. If interrupted mid-way, sidebar sees partially updated cache. Resolution: Wrap all `_meta` writes in a single transaction.

**Minor — loadDataFromFile cache staleness**:
Manual JSON import via sidebar doesn't call `refresh_caches`. Resolution: Add `invoke('refresh_caches')` after `import_json_to_db`.

**Minor — Sidebar race condition**:
`fullCounts.total > 0` watcher may miss first update if `get_full_counts` resolves before component mounts. Resolution: Add `onMounted` check alongside `watch`.

**Minor — Background count failure UX**:
If `query_image_counts` fails, `filteredCounts` currently resets to 0. Resolution: Preserve old values on error.

**Known limitation — Mutex bottleneck**:
All DB operations go through a single `OnceLock<Mutex<Connection>>`. The `refresh_caches` command will hold the Mutex for several seconds, blocking `query_images`. **RESOLVED**: Migrated to dual-connection (Reader/Writer) design — `refresh_caches` uses Writer connection, `query_images` uses Reader connection, WAL mode allows concurrent access.

---

## Work Objectives

### Core Objective
将启动和筛选性能提升到不低于纯前端方案水平，并实现收藏更新后即时增量 DB 更新。

### Concrete Deliverables
- `query_images` 不再返回任何 counts（仅图片+标签），单次调用 <50ms
- 新增 `query_image_counts` 后台执行聚合查询，不阻塞 UI
- `fullCounts` 从 `_meta` 缓存读取，启动时无需聚合查询
- 侧边栏聚合数据从 `_meta` 缓存读取，瞬时返回
- 年份筛选利用 `idx_images_created_at` 索引做范围扫描
- 新增 `idx_images_sanity_level` 索引
- `ANALYZE` 在 DB 创建后运行，优化查询计划
- 收藏更新后触发 `reimport_db`(Rust端读文件+导入+刷新缓存)，不 `location.reload()`
- **双连接（Reader/Writer）解除 Mutex 瓶颈**：`refresh_caches` 写入不阻塞图片查询

### Must Have
- `query_images` 不再做任何 COUNT/COUNT(DISTINCT) 聚合查询
- 年份筛选从 `CAST(substr(...))` 改为可索引的范围比较
- 全量 counts 缓存到 `_meta` 表，只在 import 后刷新
- 侧边栏聚合数据缓存到 `_meta` 表
- `reimport_db` 直接从磁盘读 `images.json`（不经过 IPC 传文件内容）
- 收藏更新后不 `location.reload()`
- **确保现有数据库用户也能获得新索引**（在 `ensure_db` 的 "exists" 路径也执行 `CREATE INDEX IF NOT EXISTS` + `ANALYZE`）
- **缓存读取错误处理**：`_meta` 缓存 JSON 损坏时退回到全表扫描查询
- **原子性缓存写入**：`refresh_caches()` 用单次事务写入所有 `_meta` 键
- **双连接 Reader/Writer**：`init_db()` 打开两个 SQLite 连接，读操作走 Reader 连接，写操作走 Writer 连接，WAL 模式确保两者不互斥

### Must NOT Have (Guardrails)
- 不改 `pxder/` 下任何文件（pipeline 不变）
- 不改现有数据库表结构（不新增表/列）
- 不引入 FTS5 全文搜索
- 不改 `get_filter_options` 的语义（返回侧边栏需要的年份/作者/标签列表）
- 不改 `loadEnd` 的分页判断逻辑（仍用 `result.images.length < limit`）
- 不引入连接池（`r2d2`/`deadpool`）—— 双连接方案已满足需求，不过度设计
- 不引入 Tauri async 命令重构
- 不改 Pinia store 超出最小范围（仅移除 `include_counts` + 新增 `fetchFilteredCounts`）

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.
> Acceptance criteria requiring "user manually tests/confirms" are FORBIDDEN.

### Test Decision
- **Infrastructure exists**: NO (no test framework)
- **Automated tests**: Comparison script (same as before) + manual verification via app UI
- **Framework**: Standalone script + Playwright for UI testing

### QA Policy
Every task includes agent-executed QA scenarios. Evidence saved to `.sisyphus/evidence/task-{N}-{scenario}.{ext}`.

- **Backend/Rust**: Bash — compile + run with test data
- **Frontend**: Playwright — navigate, verify image list loads, verify counts populate
- **Comparison**: Bash — compare execution time of old vs new approach

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Rust 基础设施 — 完全并行，3 任务):
├── T1: query.rs — 移除 include_counts，年份筛选改为范围查询，新增 CountsResult
├── T2a: db.rs — 双连接 Reader/Writer（解除 Mutex 瓶颈），添加 sanity_level 索引，
│                 ANALYZE，refresh_caches() 原子写入，迁移逻辑
├── T2b: db.rs — 存量函数适配：ensure_db → get_writer()，import_json_to_db_logic → get_writer()
└── T3: main.rs — 新增/重构所有 Tauri 命令，新命令中 refresh_caches 用 get_writer()

Wave 2 (后端功能完善 — T3 之后串行):
├── T3b: db.rs + main.rs — refresh_caches() 原子写入（单事务）、_meta 缓存读取回退逻辑
│                           （这两步可以合并到 T2/T3 中，但拆分出来确保原子性被独立验证）

Wave 3 (前端层 — 适配新后端，2 任务并行):
├── T4: store/index.ts — 移除 include_counts，新增 fetchFilteredCounts 后台获取
└── T5: App.vue + Sidebar.vue — 简化 init，收藏更新后 reimport_db + 刷新，竞态条件修复

Wave 4 (验证 — 1 任务 + 4 并行终审):
├── T6: 构建对比测试（旧方案 vs 新方案执行时间）
└── F1-F4: Final Verification Wave

Critical Path: T1/T2 → T3 → T4 → T5
Parallel Speedup: ~50% faster than sequential
Max Concurrent: 3 (Wave 1)
```

### Dependency Matrix
- **T1** (query.rs): none — Wave 1
- **T2** (db.rs): none — Wave 1
- **T3** (main.rs): T1, T2 — Wave 1 (depends on signatures from T1/T2, but can be drafted concurrently)
- **T4** (store): T3 — Wave 3
- **T5** (App.vue): T3, T4 — Wave 3

> Note: T1, T2a+T2b, T3 操作不同的 Rust 模块，大部分独立。T3 引用 T1/T2 的签名，但可以并行起草，最后一起编译。
> T2a 和 T2b 合并为一个 db.rs 任务执行（同一文件，连续修改）。

---

## TODOs

- [x] 1. **query.rs — 移除 include_counts + 年份筛选优化 + CountsResult**

  **What to do**:
  - 从 `ImageQuery` 中移除 `include_counts: Option<bool>` 字段
  - 新增 `CountsResult` 结构体（给新命令用）：
    ```rust
    #[derive(Debug, Serialize)]
    pub struct CountsResult {
        pub total: i64,
        pub illust_count: i64,
        pub author_count: i64,
        pub tag_count: i64,
    }
    ```
  - 保留 `build_counts_sql()`、`build_tag_count_sql()`（给 `query_image_counts` 命令用）
  - 移除 `QueryResult` 中的 `illust_count`、`author_count`、`tag_count` 字段（只保留 `images` + `total`）
    > 注意：`total` 暂时保留在 `build_sql`/`build_count_sql` 中，但后续会被彻底移除（T3 中处理）
  - 优化年份筛选：将 `CAST(substr(i.created_at,1,4) AS INTEGER) = ?` 改为可索引的范围比较
    ```rust
    // 修改前 (约 query.rs:231-241)
    let y = self.year.unwrap_or(0);
    if y == 1 {
        sql.push_str(" AND CAST(substr(i.created_at,1,4) AS INTEGER) < 2000");
    } else if y > 1 {
        let n = push_param(&mut pi, &mut params, y);
        sql.push_str(&format!(" AND CAST(substr(i.created_at,1,4) AS INTEGER) = ?{}", n));
    }

    // 修改后
    if y == 1 {
        sql.push_str(" AND i.created_at < '2000-01-01'");
    } else if y > 1 {
        let year_start = format!("{}-01-01", y);
        let year_end = format!("{}-01-01", y + 1);
        let n1 = push_param(&mut pi, &mut params, year_start);
        let n2 = push_param(&mut pi, &mut params, year_end);
        sql.push_str(&format!(" AND i.created_at >= ?{} AND i.created_at < ?{}", n1, n2));
    }
    ```
  - **为什么这么做**：`CAST(substr(created_at,1,4) AS INTEGER) = ?` 引用 `created_at` 的函数表达式，SQLite 无法使用 `idx_images_created_at` 索引。改为 `created_at >= '2024-01-01' AND created_at < '2025-01-01'` 后，SQLite 可以做高效的索引范围扫描（lexicographic comparison 对 ISO 8601 文本天然正确）。

  **Must NOT do**:
  - 不删除 `build_counts_sql()` 和 `build_tag_count_sql()`（给新命令复用）
  - 不改动 `ImageQuery` 的 filter 字段（只移除 `include_counts`）

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T2)
  - **Parallel Group**: Wave 1
  - **Blocks**: T3
  - **Blocked By**: None

  **Acceptance Criteria**:
  - [ ] `ImageQuery` 不再包含 `include_counts` 字段
  - [ ] `CountsResult` 结构体可编译
  - [ ] 年份筛选生成的 SQL 为 `AND i.created_at >= ? AND i.created_at < ?`（不是 `CAST(substr(...))`）
  - [ ] `build_counts_sql()` 和 `build_tag_count_sql()` 仍然可用
  - [ ] `QueryResult` 中移除 illust/author/tag count 字段

  **QA Scenarios**:

  ```
  Scenario: 年份筛选生成正确的 SQL
    Tool: Bash (cargo test)
    Preconditions: query.rs 编译通过
    Steps:
      1. 创建 ImageQuery { year: Some(2024), ..default() }
      2. 调用 build_sql()
      3. 打印生成的 SQL
    Expected Result: SQL 包含 "i.created_at >= '2024-01-01' AND i.created_at < '2025-01-01'"
    Evidence: .sisyphus/evidence/task-1-year-sql.txt

  Scenario: 年份 1 (before 2000) 生成正确的 SQL
    Tool: Bash
    Steps: ImageQuery { year: Some(1), ... }.build_sql()
    Expected Result: SQL 包含 "i.created_at < '2000-01-01'"
    Evidence: .sisyphus/evidence/task-1-year-before2000.txt

  Scenario: CountsResult 可序列化
    Tool: Bash (cargo build)
    Steps: cargo build
    Expected Result: 编译通过
    Evidence: .sisyphus/evidence/task-1-build.txt
  ```

  **Commit**: YES
  - Message: `perf(src-tauri): remove include_counts, optimize year filter to range query`
  - Files: `src-tauri/src/query.rs`

- [x] 2. **db.rs — 双连接 Reader/Writer + 索引 + ANALYZE + refresh_caches() + 迁移逻辑**

  **What to do**:

  **2a. 双连接解除 Mutex 瓶颈**（核心改动）：
  - 当前：`static DB: OnceLock<Mutex<Connection>>` — 单一连接，所有读/写串行
  - 改为双连接，读写分离：
    ```rust
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use rusqlite::Connection;

    static DB_READER: OnceLock<Mutex<Connection>> = OnceLock::new();
    static DB_WRITER: OnceLock<Mutex<Connection>> = OnceLock::new();

    pub fn init_db(db_path: &str) -> Result<(), String> {
        // 打开两个连接，都启用 WAL 模式
        let reader = Connection::open(db_path)
            .map_err(|e| format!("Failed to open reader connection: {e}"))?;
        reader.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to set reader pragmas: {e}"))?;

        let writer = Connection::open(db_path)
            .map_err(|e| format!("Failed to open writer connection: {e}"))?;
        writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Failed to set writer pragmas: {e}"))?;

        create_schema(&reader)?;

        DB_READER.set(Mutex::new(reader))
            .map_err(|_| "DB_READER already initialized".to_string())?;
        DB_WRITER.set(Mutex::new(writer))
            .map_err(|_| "DB_WRITER already initialized".to_string())?;

        Ok(())
    }

    /// 读操作使用此连接（query_images, get_filter_options, search_tags 等）
    pub fn get_reader() -> Result<MutexGuard<'static, Connection>, String> {
        DB_READER.get()
            .ok_or_else(|| "DB_READER not initialized".to_string())?
            .lock()
            .map_err(|e| format!("Failed to lock reader mutex: {e}"))
    }

    /// 写操作使用此连接（ensure_db, import_json_to_db, refresh_caches, reimport_db 等）
    pub fn get_writer() -> Result<MutexGuard<'static, Connection>, String> {
        DB_WRITER.get()
            .ok_or_else(|| "DB_WRITER not initialized".to_string())?
            .lock()
            .map_err(|e| format!("Failed to lock writer mutex: {e}"))
    }

    /// 向后兼容别名（默认读者连接，已使用的读命令无需修改）
    pub fn get_conn() -> Result<MutexGuard<'static, Connection>, String> {
        get_reader()
    }
    ```
  - **为什么这样有效**：SQLite WAL 模式允许多个连接并发读写。Reader 连接看到的是 Writer 连接提交后的数据快照。`refresh_caches`（写）通过 Writer 连接持锁写入时，`query_images`（读）通过 Reader 连接可以同时读取——两者不互斥。
  - **存量函数适配**：
    - `ensure_db()` 内部的写操作（import、create_indexes、写 _meta）改为 `get_writer()`
    - `import_json_to_db_logic()` 内部改为 `get_writer()`
    - 其他读函数通过 `get_conn()` / `get_reader()` 不变

  **2b. 迁移路径 + 索引 + ANALYZE**：
  - 当前 `ensure_db()` 在 "exists" 路径直接返回，不调用 `create_indexes()`
  - 这导致**现有数据库用户永远无法获得新索引** `idx_images_sanity_level`
  - 解决方案：在 `ensure_db()` 的 "exists" 路径返回前添加：
    ```rust
    // === 迁移步骤：在 "exists" 路径上也 idempotent 运行 ===
    create_indexes(&conn)?;
    conn.execute_batch("ANALYZE;")
        .map_err(|e| format!("Failed to run ANALYZE: {e}"))?;
    ```
  - 在 `create_indexes()` 中添加 `idx_images_sanity_level` 索引：
    ```rust
    CREATE INDEX IF NOT EXISTS idx_images_sanity_level ON images(sanity_level);
    ```
  - 在 `ensure_db()` import 路径末尾，`create_indexes()` 之后添加 `ANALYZE`

  **2c. 新增 `refresh_caches()` 函数（原子写入 + Writer 连接）**：
  - **使用 Writer 连接**（`let conn = get_writer()?;`），不阻塞 Reader 连接上的查询
  - 所有 `_meta` 写入在**单次事务**中完成：
    ```rust
    pub fn refresh_caches(img_dir: &str, version: Option<i64>) -> Result<(), String> {
        // Writer 连接：并行查询（Reader）不受影响
        let conn = get_writer()?;

        // 原子写入：单次事务确保所有 _meta 键同时更新或回滚
        conn.execute_batch("BEGIN TRANSACTION;")
            .map_err(|e| e.to_string())?;

        // 1. Full counts（缓存到 _meta）
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM images", [], |r| r.get(0)
        ).map_err(|e| e.to_string())?;

        let illust_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT id) FROM images", [], |r| r.get(0)
        ).map_err(|e| e.to_string())?;

        let author_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT author_id) FROM images", [], |r| r.get(0)
        ).map_err(|e| e.to_string())?;

        // 优化：对于 unfiltered 情况，tagCount = COUNT(*) FROM tags
        // 因为 ingest 逻辑确保只有在被 image_tags 引用时才创建 tags 行
        let tag_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tags", [], |r| r.get(0)
        ).map_err(|e| e.to_string())?;

        let counts_json = serde_json::json!({
            "total": total,
            "illustCount": illust_count,
            "authorCount": author_count,
            "tagCount": tag_count,
        }).to_string();

        conn.execute(
            "INSERT OR REPLACE INTO _meta(key,value) VALUES('full_counts',?1)",
            params![counts_json],
        ).map_err(|e| e.to_string())?;

        // 2. Sidebar aggregates（缓存到 _meta）
        // Years
        let mut stmt = conn.prepare(
            "SELECT CAST(substr(created_at,1,4) AS INTEGER) as year, COUNT(*) as count \
             FROM images GROUP BY year ORDER BY year DESC"
        ).map_err(|e| e.to_string())?;
        let years: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "year": row.get::<_, i64>("year")?,
                "count": row.get::<_, i64>("count")?,
            }))
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

        // Authors
        let mut stmt = conn.prepare(
            "SELECT author_id as id, author_name as name, author_account as account, \
             COUNT(DISTINCT id) as count FROM images \
             GROUP BY author_id ORDER BY count DESC LIMIT 100"
        ).map_err(|e| e.to_string())?;
        let authors: Vec<serde_json::Value> = /* same pattern */;

        // Tags
        let mut stmt = conn.prepare(
            "SELECT t.name, t.translated_name, COUNT(DISTINCT it.image_id) as count \
             FROM tags t JOIN image_tags it ON it.tag_id = t.id \
             GROUP BY t.name ORDER BY count DESC LIMIT 200"
        ).map_err(|e| e.to_string())?;
        let tags: Vec<serde_json::Value> = /* same pattern */;

        // Store sidebar caches
        conn.execute(
            "INSERT OR REPLACE INTO _meta(key,value) VALUES('sidebar_years',?1)",
            params![serde_json::to_string(&years).map_err(|e| e.to_string())?],
        ).map_err(|e| e.to_string())?;
        // ... same for authors, tags

        // 3. Update json_version if provided
        if let Some(v) = version {
            conn.execute(
                "INSERT OR REPLACE INTO _meta(key,value) VALUES('json_version',?1)",
                params![v.to_string()],
            ).map_err(|e| e.to_string())?;
        }

        // 提交事务（所有 _meta 键原子更新）
        conn.execute_batch("COMMIT;")
            .map_err(|e| e.to_string())?;

        Ok(())
    }
    ```
  - 在 `ensure_db()` 中，最后调用 `refresh_caches()` 替代重复的版本写入逻辑：
    ```rust
    // Replace the old version-storing code (lines ~297-309) with:
    refresh_caches(img_dir, version)?;
    ```

  **Must NOT do**:
  - 不新增表，仅新增 `_meta` key
  - 不改变现有 `create_schema()` 中的表结构

  **Parallelization**:
  - **Can Run In Parallel**: YES (with T1)
  - **Parallel Group**: Wave 1
  - **Blocks**: T3
  - **Blocked By**: None

  **Acceptance Criteria**:
  - [ ] `idx_images_sanity_level` 索引存在于 DB schema 中
  - [ ] `ensure_db` 后执行了 `ANALYZE`
  - [ ] `refresh_caches` 在 DB 中生成 `full_counts`、`sidebar_years`、`sidebar_authors`、`sidebar_tags` 的 `_meta` 行
  - [ ] `refresh_caches` 更新/不更新 `json_version` 取决于是否传入 `version`
  - [ ] 所有缓存的 JSON 在反序列化后与原始 SQL 查询结果一致
  - [ ] **`DB_READER` 和 `DB_WRITER` 两个连接均已初始化**，可通过 `get_reader()` 和 `get_writer()` 访问
  - [ ] **向后兼容**：`get_conn()` 作为别名返回 reader 连接，存量读命令无需修改
  - [ ] **并发验证**：`refresh_caches`（Writer）运行时，`query_images`（Reader）不被阻塞（验证：同时调用两者，比较耗时与串行执行的差异）

  **QA Scenarios**:

  ```
  Scenario: 验证 sanity_level 索引存在
    Tool: Bash
    Preconditions: images.db 存在
    Steps:
      1. sqlite3 images.db "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_images_sanity_level';"
    Expected Result: 返回 idx_images_sanity_level
    Evidence: .sisyphus/evidence/task-2-index.txt

  Scenario: 验证 cache 被写入 _meta
    Tool: Bash
    Preconditions: ensure_db 刚运行过
    Steps:
      1. sqlite3 images.db "SELECT key, substr(value,1,50) FROM _meta WHERE key LIKE 'sidebar_%' OR key='full_counts';"
    Expected Result: 返回 >= 4 行（full_counts, sidebar_years, sidebar_authors, sidebar_tags）
    Evidence: .sisyphus/evidence/task-2-cache.txt

  Scenario: 验证 ANALYZE 运行
    Tool: Bash
    Steps:
      1. sqlite3 images.db "SELECT * FROM sqlite_stat1 LIMIT 5;"
    Expected Result: 返回 > 0 行（统计信息存在）
    Evidence: .sisyphus/evidence/task-2-analyze.txt

  Scenario: 双连接初始化验证
    Tool: Bash (cargo test 或 cargo build)
    Preconditions: 应用启动，ensure_db 已调用
    Steps:
      1. 在 db.rs 中添加测试：同时获取 get_reader() 和 get_writer()
      2. 验证两个连接操作同一个 DB
    Expected Result: reader 和 writer 都可以正常查询
    Evidence: .sisyphus/evidence/task-2-dual-conn.txt

  Scenario: 读写并发不互斥
    Tool: Bash (模拟并发)
    Preconditions: DB 已创建，有 20 万行数据
    Steps:
      1. 开两个线程：线程 A 执行 refresh_caches（~2s），线程 B 同时执行 SELECT COUNT(*) FROM images
      2. 记录两个操作的完成时间
    Expected Result: B 在 A 完成前返回结果（不被 writer 阻塞）
    Evidence: .sisyphus/evidence/task-2-concurrent.txt
  ```

  **Commit**: YES (groups with T1)
  - Message: `perf(src-tauri): add sanity_level index, ANALYZE, and refresh_caches()`
  - Files: `src-tauri/src/db.rs`, `src-tauri/src/query.rs`

- [x] 3. **main.rs — 新增/重构所有 Tauri 命令**

  **What to do**:
  - **3a. `query_images` 简化**：
    - 移除 `build_count_sql()` 调用（不再查 `total`）
    - 移除 `include_counts` 相关 block
    - 只做：`build_sql()` + 多页边界处理 + 批量取标签
    - 新签名：`fn query_images(query: ImageQuery) -> Result<ImagesResult, String>`
    - 新结构体（或复用 `QueryResult` 但移除 count 字段）：
      ```rust
      #[derive(Serialize)]
      pub struct QueryResult {
          pub images: Vec<ImageRow>,
      }
      ```
    - 不再接受 `img_dir` 参数（当前代码已无此参数 — 确认签名）

  - **3b. 新增 `query_image_counts` 命令**：
    ```rust
    #[tauri::command]
    fn query_image_counts(query: query::ImageQuery) -> Result<query::CountsResult, String> {
        let conn = db::get_conn()?;

        // 单次查询获取 total + illust_count + author_count
        let (csql, cparams) = query.build_counts_sql();
        let refs: Vec<&dyn ToSql> = cparams.iter().map(|b| b.as_ref()).collect();
        let (total, illust_count, author_count): (i64, i64, i64) = conn
            .query_row(&csql, rusqlite::params_from_iter(&refs), |row| {
                Ok((row.get("total")?, row.get("illust_count")?, row.get("author_count")?))
            })
            .map_err(|e| format!("Counts query failed: {e}"))?;

        // 单独查 tag count（需要三重 JOIN）
        let (tcsql, tcparams) = query.build_tag_count_sql();
        let tcrefs: Vec<&dyn ToSql> = tcparams.iter().map(|b| b.as_ref()).collect();
        let tag_count: i64 = conn
            .query_row(&tcsql, rusqlite::params_from_iter(&tcrefs), |row| row.get(0))
            .map_err(|e| format!("Tag count query failed: {e}"))?;

        Ok(query::CountsResult { total, illust_count, author_count, tag_count })
    }
    ```

  - **3c. 新增 `get_full_counts` 命令（从 _meta 缓存读取）**：
    ```rust
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
    ```

  - **3d. 新增 `refresh_caches` 命令（前端触发）**：
    ```rust
    #[tauri::command]
    fn refresh_caches(img_dir: String, version: Option<i64>) -> Result<(), String> {
        db::refresh_caches(&img_dir, version)
    }
    ```

  - **3e. 新增 `reimport_db` 命令（收藏更新后调用）**：
    ```rust
    #[tauri::command]
    fn reimport_db(img_dir: String, version: Option<i64>) -> Result<db::ImportResult, String> {
        let json_path = format!("{img_dir}/data/images.json");
        let json_content = std::fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read images.json: {e}"))?;
        let result = db::import_json_to_db_logic(&img_dir, &json_content)?;
        db::refresh_caches(&img_dir, version)?;
        Ok(result)
    }
    ```

  - **3f. `get_filter_options` 改为从 _meta 缓存读取 + 回退逻辑**：
    ```rust
    #[tauri::command]
    fn get_filter_options() -> Result<query::FilterOptions, String> {
        let conn = db::get_conn()?;

        // 从缓存读取。如果 JSON 损坏或不存在，回退到全表扫描（log 警告）
        fn read_json<T: serde::de::DeserializeOwned>(conn: &Connection, key: &str,
            fallback: fn(&Connection) -> Result<T, String>
        ) -> Result<T, String> {
            let val: String = match conn.query_row(
                "SELECT value FROM _meta WHERE key = ?1",
                params![key],
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

        // 回退函数：直接从 DB 聚合查询（与原来一致）
        fn fallback_years(conn: &Connection) -> Result<Vec<serde_json::Value>, String> { ... }
        fn fallback_authors(conn: &Connection) -> Result<Vec<serde_json::Value>, String> { ... }
        fn fallback_tags(conn: &Connection) -> Result<Vec<serde_json::Value>, String> { ... }

        Ok(query::FilterOptions {
            years: read_json(&conn, "sidebar_years", fallback_years)?,
            authors: read_json(&conn, "sidebar_authors", fallback_authors)?,
            tags: read_json(&conn, "sidebar_tags", fallback_tags)?,
        })
    }
    ```

  - **3g. `ensure_db` 增加迁移路径**：
    ```rust
    // 在 ensure_db() 返回"exists"之前（schema_version 检查后），添加：
    // 幂等运行 create_indexes() + ANALYZE，确保现有用户获得新索引
    create_indexes()?;
    conn.execute_batch("ANALYZE;")
        .map_err(|e| format!("Failed to run ANALYZE: {e}"))?;
    ```

  - 注册所有新命令到 `invoke_handler!`

  **Must NOT do**:
  - 不删除 `import_json_to_db` 命令（用户手动导入文件仍需此功能）
  - 不修改 `ensure_db` 的签名（仍然需要它做首次初始化）

  **Parallelization**:
  - **Can Run In Parallel**: NO (与 T1/T2 同进程序贯)
  - **Parallel Group**: Wave 2 (but sequential — same file)
  - **Blocked By**: T1, T2
  - **Blocks**: T4, T5

  **Acceptance Criteria**:
  - [ ] `query_images` 返回的数据中不包含 `illust_count`、`author_count`、`tag_count`
  - [ ] `query_image_counts` 返回正确的 total/illust_count/author_count/tag_count
  - [ ] `get_full_counts` 从 `_meta` 读取 full_counts JSON，字段名为 `total`/`illustCount`/`authorCount`/`tagCount`
  - [ ] `reimport_db` 在不传入 JSON 内容的情况下，直接从 `images.json` 文件导入并刷新缓存
  - [ ] `refresh_caches` 更新 `json_version`（当传入 version 时）
  - [ ] `get_filter_options` 不再执行聚合 SQL，而是从 `_meta` 读取
  - [ ] **`get_filter_options` 缓存回退**：`_meta` 中 `sidebar_years` 内容为无效 JSON 时，退回到全表扫描查询，不 crash
  - [ ] **`ensure_db` 迁移路径**：现有 images.db 中不存在 `idx_images_sanity_level` 索引时，启动后会创建它
  - [ ] 所有命令在 `invoke_handler!` 中注册

  **QA Scenarios**:

  ```
  Scenario: query_images 返回无 counts
    Tool: Bash (通过 cargo test 或 Tauri invoke)
    Preconditions: 应用运行，DB 已创建
    Steps:
      1. 调用 query_images({ limit: 5, offset: 0 })
    Expected Result: 返回 { images: [...] }，不包含 total/illust_count 等字段
    Evidence: .sisyphus/evidence/task-3-no-counts.json

  Scenario: query_image_counts 返回正确值
    Tool: Bash
    Steps:
      1. 调用 query_image_counts({})  // 无筛选条件
      2. sqlite3 images.db "SELECT COUNT(*) FROM images"
      3. 对比 total 字段
    Expected Result: total == COUNT(*) from SQLite
    Evidence: .sisyphus/evidence/task-3-counts.json

   Scenario: get_filter_options 从缓存读取
    Tool: Bash
    Steps:
      1. 手动修改 _meta 中 sidebar_years 的值
      2. 调用 get_filter_options()
    Expected Result: 返回值反映手动修改后的值
    Evidence: .sisyphus/evidence/task-3-cached-filters.json

  Scenario: get_filter_options 缓存损坏回退
    Tool: Bash
    Preconditions: _meta 中 sidebar_years 值为无效 JSON（如 "not json"）
    Steps:
      1. sqlite3 images.db "INSERT OR REPLACE INTO _meta(key,value) VALUES('sidebar_years','not json');"
      2. 调用 get_filter_options()
    Expected Result: 不 crash，返回从 DB 聚合查询得到的 year 列表（stderr 输出 [WARN]）
    Evidence: .sisyphus/evidence/task-3-cache-fallback.txt

  Scenario: ensure_db 创建迁移索引
    Tool: Bash
    Preconditions: 一个旧版 images.db（无 idx_images_sanity_level 索引）
    Steps:
      1. sqlite3 images.db "DROP INDEX IF EXISTS idx_images_sanity_level;"
      2. 确保应用下次启动时触发 ensure_db（touch images.json 或重启）
      3. 调用 ensure_db（模拟启动）
      4. sqlite3 images.db "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_images_sanity_level';"
    Expected Result: idx_images_sanity_level 存在
    Evidence: .sisyphus/evidence/task-3-migration-index.txt

  Scenario: reimport_db 端到端
    Evidence: .sisyphus/evidence/task-3-reimport.json
  ```

  **Commit**: YES
  - Message: `perf(src-tauri): de-countify query_images, add dedicated count/refresh/reimport commands`
  - Files: `src-tauri/src/main.rs`

- [x] 4. **store/index.ts — 移除 include_counts，新增后台 counts 获取**

  **What to do**:
  - 移除 `buildFilterQuery()` 中与 `include_counts` 相关的逻辑（该字段已被 Rust 端移除）
  - 修改 `loadImagesByPage()`：
    ```typescript
    async loadImagesByPage(isFirstLoad = false) {
      if (isFirstLoad) {
        this.curPageCursor = 0
        this.loadEnd = false
        this.imagesFiltered = []
      }
      const query = this.buildFilterQuery()
      query.offset = this.curPageCursor
      query.limit = 60
      query.sort_by = this.masonryConfig.imageSortBy
      // 不再设置 include_counts
      const result = await invoke<any>('query_images', { query })
      // ... 转换 images（不变） ...
      if (isFirstLoad) {
        this.imagesFiltered = images
      } else {
        this.imagesFiltered = this.imagesFiltered.concat(images)
      }
      this.curPageCursor += result.images.length
      this.loadEnd = result.images.length < (query.limit ?? 60)
      // 不再设置 filteredCounts 在这里 —— 交给后台任务
      
      // 首次加载或重新筛选后，触发后台 counts 获取
      if (isFirstLoad || this.curPageCursor === 0) {
        this.fetchFilteredCounts()
      }
    }
    ```
  - 新增 `fetchFilteredCounts()` action（失败时保留旧值）：
    ```typescript
    async fetchFilteredCounts() {
      const query = this.buildFilterQuery()
      try {
        const result = await invoke<any>('query_image_counts', { query })
        this.filteredCounts.total = result.total
        this.filteredCounts.illustCount = result.illust_count
        this.filteredCounts.authorCount = result.author_count
        this.filteredCounts.tagCount = result.tag_count
      } catch (e) {
        // 失败时保留旧值（不重置为 0），仅打印警告
        console.warn('query_image_counts failed (preserving previous values):', e)
      }
    }
    ```
  - 更新 `QueryResult` 接口（移除 counts 字段）：
    ```typescript
    export interface QueryResult {
      images: Image[]
      // total, illust_count, author_count, tag_count → 移除
    }
    ```
  - 移除 `QueryResult` 中不再使用的接口字段
  - 可新增 `CountsResult` 接口（可选，直接用 `any` 也可以）
    ```typescript
    export interface CountsResult {
      total: number
      illust_count: number
      author_count: number
      tag_count: number
    }
    ```

  **Must NOT do**:
  - 不改变 `imagesFiltered` 的更新逻辑（仍然用 `shallowRef` + replace）
  - 不改变 `buildFilterQuery()` 现有的 filter 字段映射

  **Parallelization**:
  - **Can Run In Parallel**: NO (与 T5 同文件)
  - **Parallel Group**: Wave 3
  - **Blocked By**: T3
  - **Blocks**: T5

  **Acceptance Criteria**:
  - [ ] `loadImagesByPage` 不设置 `include_counts` 字段
  - [ ] `fetchFilteredCounts` 在首次加载或筛选变化后被调用
  - [ ] `fetchFilteredCounts` 完成前，`filteredCounts` 保持旧值（不显示错误的状态）
  - [ ] `filteredCounts` 在 `fetchFilteredCounts` 完成后正确更新

  **QA Scenarios**:

  ```
  Scenario: 首页加载不阻塞 counts
    Tool: Playwright
    Preconditions: 应用启动，DB 已创建
    Steps:
      1. 打开应用
      2. 观察图片列表渲染
      3. 观察侧边栏"筛选"行 counts
    Expected Result: 图片列表先渲染（瞬间），counts 晚几秒后出现
    Evidence: .sisyphus/evidence/task-4-async-counts.png
  ```

  **Commit**: YES (groups with T5)
  - Message: `perf(web): remove include_counts, add async fetchFilteredCounts action`
  - Files: `web/src/store/index.ts`

- [x] 5. **App.vue — 精简 init + 收藏更新后即时导入不 reload**

  **What to do**:

  **5a. 精简 `init()`**：
  ```typescript
  async function init() {
    try {
      loading.value = true
      store.curPageCursor = 0
      store.loadEnd = false
      store.imagesFiltered = []
      if (__CONFIG__.imgDir) {
        if (store.masonryConfig.loadImageByLocalHttp) {
          if (!sessionStorage.getItem('local_server_started')) {
            await invoke('start_local_server', { base: __CONFIG__.imgDir })
            sessionStorage.setItem('local_server_started', 'true')
          }
        }
        // (1) 确保 DB 存在（快速——只是检查并打开连接）
        const ensureResult = await invoke('ensure_db', {
          imgDir: __CONFIG__.imgDir,
          version: Number(localStorage.getItem('_images_json_version') || '0'),
        })
        // (2) 读取缓存的 fullCounts（从 _meta，无 SQL 聚合查询）
        if (ensureResult.status !== 'no_data') {
          try {
            const counts = await invoke<any>('get_full_counts')
            store.fullCounts.total = counts.total
            store.fullCounts.illustCount = counts.illustCount
            store.fullCounts.authorCount = counts.authorCount
            store.fullCounts.tagCount = counts.tagCount
          } catch (e) {
            console.warn('Full counts not cached yet:', e)
          }
        }
        // (3) 加载第一页图片（快速——无 counts）
        await store.loadImagesByPage(true)
        // sidebar 的 getFilters 通过 watcher 读取 fullCounts.total
        // get_filter_options 现在从 _meta 缓存读取，瞬间返回
      }
    } catch (e) {
      console.error(e)
      const msg = (e as Error).message || JSON.stringify(e)
      showModalMsg.value = true
      modalMsg.value += `<br><div style="color:#ff6565">${msg}</div>`
    } finally {
      loading.value = false
      setTimeout(() => { isInit.value = true }, 500)
    }
  }
  ```

  **5b. 收藏更新后即时导入不 reload**：
  ```typescript
  // 在 startCmd.on('close', ...) 中：
  startCmd.on('close', async data => {
    const msg = `UpdateBookmark command finished with code ${data.code}...`
    modalMsg.value += msg

    try {
      modalMsg.value += '\n开始导入数据到数据库...\n'
      const version = (Number(localStorage.getItem('_images_json_version') || '0')) + 1
      localStorage.setItem('_images_json_version', String(version))

      // reimport_db: Rust 端直接读文件（不经过 IPC 传 JSON），增量导入 + 刷新缓存
      const result = await invoke<{ imported: number; skipped: number }>('reimport_db', {
        imgDir: __CONFIG__.imgDir,
        version,
      })
      modalMsg.value += `导入完成：${result.imported} 条新数据，${result.skipped} 条跳过\n`

      // 刷新前端 counts
      const counts = await invoke<any>('get_full_counts')
      store.fullCounts = counts

      // 刷新侧边栏（get_filter_options 已从缓存读取）
      // sidebar 通过 watcher 自动触发

      // 刷新第一页
      await store.loadImagesByPage(true)

      pendingReload = false  // 不再需要 reload
      modalMsg.value += '数据库已更新\n'
    } catch (err) {
      modalMsg.value += `<br><div style="color:#ff6565">导入失败: ${err}</div>`
      pendingReload = true  // fallback: 下次启动时重新导入
    }
  })
  ```

  **5c. 关闭弹窗不再触发 reload**：
  ```typescript
  function closeMsgModal() {
    showModalMsg.value = false
    modalMsg.value = ''
    killUpdateCp()
    // 不再需要 pendingReload → location.reload()
    // 用户已经看到更新后的数据了
  }
  ```

  **Sidebar.vue 微调**：
  - **竞态条件修复**：`fullCounts.total > 0` 的 `watch` 可能在 `get_full_counts` 在组件挂载前完成时错过首次更新。添加 `onMounted` 检查：
    ```typescript
    // 替换现有的 watch(fullCounts...) 为：
    watch(() => store.fullCounts.total, (newVal) => {
      if (newVal > 0) getFilters()
    })

    onMounted(() => {
      // 处理 watch 在 mount 前完成的情况（`get_full_counts` 在 init 中已调用）
      if (store.fullCounts.total > 0) {
        getFilters()
      }
    })
    ```
  - **`loadDataFromFile` 后刷新缓存**：在 `loadDataFromFile` 函数中（Sidebar.vue 大约第 555 行），在 `store.loadImagesByPage(true)` 之后添加：
    ```typescript
    // 手动导入 JSON 后，刷新 _meta 缓存
    import { invoke } from '@tauri-apps/api/tauri'
    // ... loadDataFromFile 代码 ...
    await store.loadImagesByPage(true)
    await invoke('refresh_caches', { imgDir: __CONFIG__.imgDir })  // 新增
    // ... refresh sidebar counts ...
    ```
  - `get_filter_options` 现在从缓存读取，响应速度应该已从 1-3s 降至 <10ms

  **Must NOT do**:
  - 不改变 `fetchMore` 函数（仍然正常调用 `loadImagesByPage()`）
  - 不删除 `isInit` + 500ms 的延迟保护（防止启动时 watcher 误触发）
  - 不删除 `import_json_to_db` 相关的前端代码（手动导入文件仍然需要前端读文件）

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (with T4)
  - **Blocked By**: T3, T4

  **Acceptance Criteria**:
  - [ ] `init()` 只调用 1 次 `loadImagesByPage`（原来 2 次 `query_images`）
  - [ ] `init()` 不再调用 `query_images` 带 `include_counts`
  - [ ] 启动后 ~500ms 内侧边栏"总计"行显示正确数值
  - [ ] 收藏更新完成后，不 `location.reload()` 且图片列表自动刷新
  - [ ] 收藏更新完成后，侧边栏 counts 更新
  - [ ] `closeMsgModal()` 不触发 `location.reload()`
  - [ ] **Sidebar 竞态条件修复**：即使 `get_full_counts` 在 Sidebar 组件 mount 前完成，侧边栏也能正确触发 `getFilters()`
  - [ ] **loadDataFromFile 刷新缓存**：手动导入 JSON 文件后，侧边栏 counts 能正确更新

  **QA Scenarios**:

  ```
  Scenario: 启动体验验证
    Tool: Playwright
    Preconditions: images.db 已存在
    Steps:
      1. 打开应用
      2. 记录从点击到图片出现的时间
    Expected Result: 图片在 <1s 内出现，侧边栏 counts 在 <3s 内更新
    Evidence: .sisyphus/evidence/task-5-startup-speed.png

  Scenario: 收藏更新后不 reload
    Tool: Playwright
    Preconditions: 应用正在运行
    Steps:
      1. 触发更新收藏
      2. 等待完成
      3. 检查页面是否被刷新（观察页面状态连续）
    Expected Result: 完成后页面不刷新，图片列表和 counts 更新为新数据
    Evidence: .sisyphus/evidence/task-5-no-reload.png

  Scenario: Sidebar 竞态条件——get_full_counts 在 mount 前完成
    Tool: Playwright
    Preconditions: 应用已加载，DB 已创建
    Steps:
      1. 在 Sidebar.vue 的 setup 中添加 console.log 标记来追踪 onMounted 时序
      2. 确保 init() 中 get_full_counts 在 Sidebar mount 前解析
      3. 观察 getFilters() 是否被调用
    Expected Result: getFilters() 被调用（通过 onMounted 守卫），侧边栏正确显示年份/作者/标签
    Evidence: .sisyphus/evidence/task-5-race-condition.txt

  Scenario: loadDataFromFile 刷新缓存
    Tool: Playwright
    Preconditions: 应用运行中
    Steps:
      1. 通过 UI 上传新的 images.json
      2. 等待导入完成
      3. 检查侧边栏 counts 是否反映新数据
    Expected Result: 侧边栏 counts 在导入后更新，无需手动刷新页面
    Evidence: .sisyphus/evidence/task-5-loaddata-cache.png
  ```

  **Commit**: YES (groups with T4)
  - Message: `perf(web): simplify init with cached counts, add post-update reimport without reload`
  - Files: `web/src/App.vue`, `web/src/components/Sidebar/Sidebar.vue`

---

### Final Verification Wave

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns. Check evidence files.
  **新增检查**：验证迁移路径已实现（`ensure_db` 的 "exists" 路径有 `create_indexes()`），验证缓存回退逻辑存在（`get_filter_options` 中 JSON 解析失败时退回到全扫描）。
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Migration [OK/FAIL] | Cache Fallback [OK/FAIL] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build` + `vue-tsc --noEmit` (in web/). Check for: dead code, error handling, any `unwrap()` that could panic in commands. **特别注意**：`refresh_caches` 中的事务处理（是否有 ROLLBACK 处理？），缓存回退中的警告日志。
  Output: `Build [PASS/FAIL] | TypeScript [PASS/FAIL] | Transaction Safety [OK/FAIL] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Execute QA scenarios from every task. Test cross-task integration: startup flow (init → ensure_db with migration → get_full_counts → loadImagesByPage), filter flow (query_images first → query_image_counts arrives later), post-update flow, **cache corruption fallback**, **sidebar race condition**.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N/N] | VERDICT`

- [x] F4. **Performance Benchmark** — `unspecified-high`
  Create a benchmark script (or manual test):
  1. Start timing from app launch
  2. Record time to first image visible
  3. Record time to counts populated
  4. Record time for filter operations
  5. Repeat 3 times, report averages
  Compare against the OLD pure-JS approach timing.
  Output: `Startup [X.Xs] | Filter [X.Xs] | VERDICT: BETTER/WORSE vs old approach`

---

## Commit Strategy

- **T1+T2**: `perf(src-tauri): remove include_counts, add indexes, ANALYZE, and refresh_caches()`
- **T3**: `perf(src-tauri): de-countify query_images, add counts/reimport/refresh commands`
- **T4+T5**: `perf(web): async counts flow, cached init, post-update reimport without reload`

---

## Success Criteria

### Verification Commands
```bash
# Rust 编译
cd src-tauri && cargo build

# 前端类型检查
cd web && vue-tsc --noEmit

# 验证缓存存在
sqlite3 images.db "SELECT key, substr(value,1,30) FROM _meta WHERE key LIKE 'sidebar_%' OR key='full_counts';"
# Expected: full_counts | {"total":200000,...
#           sidebar_years | [{"year":2024,...
#           sidebar_authors | [{"id":123,...
#           sidebar_tags | [{"name":"...",...

# 验证索引存在
sqlite3 images.db "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_images_sanity_level';"
# Expected: idx_images_sanity_level

# 验证 ANALYZE
sqlite3 images.db "SELECT COUNT(*) FROM sqlite_stat1;"
# Expected: > 0
```

### Final Checklist
- [x] 所有 "Must Have" 已实现
- [x] 所有 "Must NOT Have" 未违反
- [x] `query_images` 不再返回任何 counts
- [x] `query_image_counts` 在后台运行，不阻塞 UI
- [x] 年份筛选使用 `created_at >= ? AND created_at < ?`
- [x] 侧边栏 aggregates 从 `_meta` 读取
- [x] 收藏更新后 `reimport_db` 触发，无 `location.reload()`
- [x] 现有数据库用户通过 `ensure_db` "exists" 路径获得新索引和 ANALYZE
- [x] 缓存 JSON 损坏时，`get_filter_options` 退回到全表扫描查询，不 crash
- [x] `refresh_caches()` 使用单次事务写入所有 `_meta` 键

---

## Edge Cases & Observations

- **`get_filter_options` 缓存最大过时窗口**：缓存数据在 `reimport_db`/`import_json_to_db` 完成到 `refresh_caches` 运行之间可能过时。当前设计中此窗口为 ~100ms。可以接受。
- **WAL 文件增长**：双连接模式下 WAL 文件可能比单连接增长更快（两个连接各维护自己的 WAL）。Reader 连接在空闲时自动 checkpoint，Writer 连接在事务提交时 checkpoint。正常情况下影响可忽略。
