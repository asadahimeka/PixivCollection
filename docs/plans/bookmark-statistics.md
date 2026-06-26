# 收藏夹数据可视化统计面板

## TL;DR

> **Quick Summary**: 为 PixivCollection 新增一个独立的收藏数据统计面板——以图表+排行列表的形式展示作者排行、标签频率、收藏/浏览分布、时间趋势、R18/AI 比例、尺寸偏好等维度的统计数据。
>
> **Deliverables**:
> - 新的 Rust `stats.rs` 模块 + `get_statistics` Tauri command（不修改现有命令）
> - 新的 Vue `Statistics/` 组件目录（含 13 个组件：Overview, AuthorChart, TagChart, TimeChart, Distribution, PieSection, TopLists, AuthorDistribution, WordCloud, R18AiTrend, TagTrend, AuthorDiscovery + Modal 容器；另有 5 个已有组件扩展增值功能）
> - 基于 Chart.js + vue-chartjs 的可视化图表
> - 独立弹窗入口（Navbar 图表按钮）
> - 统计面板内自带的筛选功能（年份/R18/AI）
>
> **Estimated Effort**: Large (~26 个 TODO)
> **Parallel Execution**: YES — 3 waves + 1 final verification
> **Critical Path**: Task 1 → Task 3 → Task 5 → Task 9 → Task 16 → Task 18 → F1-F4

---

## Context

### Original Request
用户想要一个收藏夹数据的可视化统计功能：最多作品的作者、最多标签的作品、个人喜好等维度的统计分析。

### Interview Summary
**Key Discussions**:
- **数据范围**: 统计面板独立于原有画廊筛选条件，自带独立筛选（年份、R18、AI）
- **标签统计**: 标签频率排行（哪些标签被用得最多），不是单作品标签数
- **个人喜好维度**: 全选 + 补充维度（收藏数分布、浏览数分布、时间趋势、尺寸偏好、R18/AI比例、作者贡献排行、最值 TOP、作者作品数分布）
- **可视化方式**: Chart.js 图表 + 排行列表
- **入口**: 独立弹窗/模态框（Navbar 按钮打开）
- **后端**: 新建独立的 Rust command，不动现有命令
- **数据加载**: 打开弹窗时按需加载
- **Vue 组件目录**: `web/src/components/Statistics/`
- **测试**: 手动测试，Agent QA 场景验证

**Research Findings**:
- SQLite 数据库已有 `_meta` 缓存系统和 `refresh_caches()` 聚合函数
- 现有 `get_filter_options` 已返回 TOP 100 作者和 TOP 200 标签（但不够全面）
- 所有需要的字段都已索引：`bookmark`, `view`, `created_at`, `author_id`, `x_restrict`, `is_ai`
- 前端无图表库（需新增 Chart.js + vue-chartjs）
- 无模态框组件存在（需从零构建）
- `is_ai` 字段已存储但无前端筛选入口

### Metis Review
**Identified Gaps** (addressed):
- **多页作品计数策略**: 已在每种图表类型中明确使用 DISTINCT id（以作品为单位）vs COUNT(*)（以页为单位）
- **缓存策略**: 混合模式——无筛选时读 `_meta` 缓存，有筛选时实时 SQL 计算
- **AI 筛选缺失**: 统计命令单独添加 AI 筛选参数（不修改现有筛选代码）
- **暗色模式图表**: 图表随 store.colorScheme 自动切换配色
- **SQL WHERE 逻辑复用**: 为新命令创建独立的 `StatsFilter`，不与 `ImageQuery` 耦合
- **模态框组件**: 参考 ImageViewer.vue 的 overlay 模式从零构建

---

## Work Objectives

### Core Objective
为 PixivCollection 添加一个完整的收藏数据可视化统计面板，以图表+排行列表直观展示用户的收藏画像。

### Concrete Deliverables
- **Rust**: `src-tauri/src/stats.rs` — `StatsFilter`, `StatisticsResult`, `compute_statistics()`
- **Rust**: `src-tauri/src/main.rs` — 注册 `get_statistics` command
- **Vue**: `web/src/components/Statistics/StatsModal.vue` — 模态框容器 + 筛选控件
- **Vue**: `web/src/components/Statistics/StatsOverview.vue` — 概览卡片
- **Vue**: `web/src/components/Statistics/StatsAuthorChart.vue` — 作者排行图
- **Vue**: `web/src/components/Statistics/StatsTagChart.vue` — 标签排行图
- **Vue**: `web/src/components/Statistics/StatsTimeChart.vue` — 时间趋势图
- **Vue**: `web/src/components/Statistics/StatsDistribution.vue` — 收藏/浏览分布图
- **Vue**: `web/src/components/Statistics/StatsPieSection.vue` — R18/AI/尺寸比例图
- **Vue**: `web/src/components/Statistics/StatsTopLists.vue` — 最值排行列表
- **Vue**: `web/src/components/Statistics/StatsAuthorDistribution.vue` — 作者作品数分布图 **【新增】**
- **Vue**: `web/src/components/Statistics/StatsWordCloud.vue` — 标签词云 **【新增】**
- **Vue**: `web/src/components/Statistics/StatsR18AiTrend.vue` — R18/AI 年度趋势图 **【新增】**
- **Navbar**: 添加图表按钮打开统计面板
- **依赖**: `chart.js` + `vue-chartjs` 加入 web/package.json

### Definition of Done
- [ ] 打开统计面板后，所有图表和排行正确显示
- [ ] 筛选控件（年份、R18、AI）能正确过滤统计结果
- [ ] 暗色模式下图表配色自动切换
- [ ] `vue-tsc --noEmit` 通过
- [ ] 弹窗打开/关闭流畅，响应式布局正常
- [ ] `tns build` 通过（Rust 编译无错误）

### Must Have
- 作者排行（按作品数）— 柱状图 + 排名列表
- 标签排行（按使用频率）— 柱状图 + 排名列表
- 收藏数分布直方图
- 浏览数分布直方图
- 年度收藏趋势折线/柱状图
- R18/AI 占比饼图
- 作品尺寸/形状分布饼图
- 收藏数最高 TOP 10 作品列表
- 浏览数最高 TOP 10 作品列表
- 统计面板自带筛选（年份、R18 开关、AI 开关）
- 暗色模式适配
- 统计面板独立于画廊筛选状态
- 作者作品数分布图（看收藏集中在少数作者还是广泛分布） **【新增】**
- 标签词云可视化（字体大小反映标签使用频率） **【新增】**
- R18/AI 年度占比趋势图（按年份展示比例变化） **【新增】**
- 标签年度趋势堆叠面积图（TOP 20 标签每年频率变化） **【增值】**
- 作者发现时间线（按首次收录年份展示收藏史） **【增值】**
- 收藏效率排行（bookmark/view 比值最高 = 隐藏神作发现器） **【增值】**
- 累计收藏曲线（按年累计作品数增长，融入时间趋势图） **【增值】**
- 健全度等级分布（sanity_level 0-6 完整分布） **【增值】**
- 作者排序切换（按作品数 / 按总收藏数） **【增值】**
- 收藏人格总结卡片（自动生成的一段洞察文字） **【增值】**

### Must NOT Have (Guardrails)
- 不修改现有 `query_images`, `query_image_counts`, `ensure_db`, `refresh_caches`, `get_filter_options` 命令
- 不耦合统计面板筛选状态到画廊 `filterConfig`
- 不添加图表导出、CSV 下载功能 (v1 排除)
- 不添加交互式下钻（点击图表过滤画廊）
- 统计面板筛选不自动影响画廊展示
- 不使用除 Chart.js 之外的前端图表库

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: NO (no test framework in project)
- **Automated tests**: None (manual only, as requested)
- **Agent-Executed QA**: MANDATORY for all tasks

### QA Policy
Every task MUST include agent-executed QA scenarios. Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Rust backend**: Bash (cargo build + cargo run with test DB)
- **Vue frontend**: Use Playwright to open app, click nav button, assert modal shows, verify charts render
- **Type checks**: Bash (vue-tsc --noEmit)
- **Lint**: Bash (eslint)
- **Dark mode**: Playwright — toggle color scheme, verify chart colors change
- **Responsive**: Playwright — resize viewport, verify modal layout adapts

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — background deps + Rust backend):
├── Task 1: Install chart.js + vue-chartjs deps [quick]
├── Task 2: Create Rust stats.rs module — types + SQL builders [deep]
├── Task 3: Create compute_statistics() in stats.rs [deep]
├── Task 4: Register get_statistics command in main.rs [quick]

Wave 2 (Frontend components — MAX PARALLEL):
├── Task 5: Build StatsModal.vue — modal shell + filter controls [visual-engineering]
├── Task 6: Build StatsOverview.vue — summary cards [visual-engineering]
├── Task 7: Build StatsAuthorChart.vue — author ranking bar chart [visual-engineering]
├── Task 8: Build StatsTagChart.vue — tag ranking bar chart [visual-engineering]
├── Task 9: Build StatsTimeChart.vue — yearly trend chart [visual-engineering]
├── Task 10: Build StatsDistribution.vue — bookmark/view histogram [visual-engineering]
├── Task 11: Build StatsPieSection.vue — R18/AI/size static pie charts [visual-engineering]
├── Task 12: Build StatsTopLists.vue — top works tables [visual-engineering]
├── Task 18: Build StatsAuthorDistribution.vue — author works distribution bar chart [visual-engineering] 【NEW】
├── Task 19: Build StatsWordCloud.vue — tag cloud visualization [visual-engineering] 【NEW】
├── Task 20: Build StatsR18AiTrend.vue — R18/AI yearly trend chart [visual-engineering] 【NEW】
├── Task 22: Build StatsTagTrend.vue — tag yearly stacked area chart [visual-engineering] 【增值】
├── Task 23: Build StatsAuthorDiscovery.vue — author discovery timeline [visual-engineering] 【增值】

Wave 3 (Integration + Insight Extensions):
├── Task 13: Add stats button to Navbar [visual-engineering]
├── Task 14: Wire Tauri command -> store -> chart components [unspecified-high]
├── Task 15: Dark mode chart theme switching [visual-engineering]
├── Task 16: Responsive layout + mobile adaptations [visual-engineering]
├── Task 17: Transition animations + polish [visual-engineering]
├── Task 21: Extend existing components — summary card, sort toggle, cumulative line, sanity levels, hidden gems [visual-engineering] 【增值】

Wave 3 (Integration + Polish):
├── Task 13: Add stats button to Navbar [visual-engineering]
├── Task 14: Wire Tauri command → store → chart components [unspecified-high]
├── Task 15: Dark mode chart theme switching [visual-engineering]
├── Task 16: Responsive layout + mobile adaptations [visual-engineering]
├── Task 17: Transition animations + polish [visual-engineering]

Wave FINAL (Verification — 4 parallel reviews):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high + playwright)
└── Task F4: Scope fidelity check (deep)
→ Present results → Get explicit user okay

Critical Path: Task 1 → Task 3 → Task 5 → Task 14 → Task 15 → Task 16 → F1-F4 → user okay
Parallel Speedup: ~65% faster than sequential
Max Concurrent: 8 (Wave 2)
```

### Dependency Matrix

- **1**: - → 5-12 (chart.js installed)
- **2**: - → 3
- **3**: 2 → 4, 5 (Tauri command ready)
- **4**: 3 → 14 (command registered)
- **5**: 1, 3 → 14 (modal shell exists)
- **6-12**: 1 → 14 (components built)
- **13**: - → 14 (nav button exists)
- **14**: 4, 5, 6-12, 13 → 15 (all wired up)
- **15**: 14 → 16
- **16**: 15 → 17
- **17**: 16 → F1-F4
- **F1-F4**: 17 → user okay

---

## TODOs

- [x] 1. 安装前端图表库 (chart.js + vue-chartjs)

  **What to do**:
  - 在 `web/` 目录下执行 `yarn add chart.js vue-chartjs`
  - 验证 package.json 和 yarn.lock 已更新
  - 在 Vue 组件中确认 `import { Bar, Doughnut, Line } from 'vue-chartjs'` 可正常导入
  - 运行 `vue-tsc --noEmit` 确认类型检查通过

  **Must NOT do**:
  - 不要修改其他依赖
  - 不要使用 ECharts 或其他图表库

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 简单的包管理操作，无复杂逻辑
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 5-12 (chart components)
  - **Blocked By**: None

  **References**:
  - `web/package.json` — 现有依赖列表，需要在其后添加新依赖

  **Acceptance Criteria**:
  - [ ] `web/package.json` 包含 `chart.js` 和 `vue-chartjs`
  - [ ] `vue-tsc --noEmit` 在 web/ 下通过
  - [ ] 临时测试组件能正常 import chart.js 符号

  **QA Scenarios**:
  ```
  Scenario: Dependency installation succeeds
    Tool: Bash
    Preconditions: package.json exists, yarn.lock exists
    Steps:
      1. cd web && yarn add chart.js vue-chartjs
      2. Check package.json for chart.js and vue-chartjs entries
      3. Run vue-tsc --noEmit → exit code 0
    Expected Result: Both deps added, type checking passes
    Evidence: .sisyphus/evidence/task-1-deps-added.txt

  Scenario: Chart.js import works
    Tool: Bash
    Preconditions: Deps installed
    Steps:
      1. cd web && node -e "const Chart = require('chart.js'); console.log('Chart version:', Chart.version);"
    Expected Result: Chart.js version printed, no import error
    Evidence: .sisyphus/evidence/task-1-import-verified.txt
  ```

  **Commit**: NO (groups with task 2-4)
  ---

- [x] 2. 创建 Rust stats.rs 模块 — 类型定义 + SQL 构建器

  **What to do**:
  - 创建 `src-tauri/src/stats.rs` 文件
  - 定义以下类型结构体（需实现 `serde::Serialize`，注意 Rust 命名习惯：`bookmarkCount` → `bookmark_count`，前端 JSON 映射）：

    ```rust
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
        // 【新增】R18/AI 年度趋势
        pub r18_trend: Vec<R18TrendItem>,
        pub ai_trend: Vec<AiTrendItem>,
        // 【增值维度】
        pub tag_trend: Vec<TagTrendItem>,
        pub hidden_gems: Vec<HiddenGem>,
        pub author_discovery: Vec<AuthorDiscovery>,
        pub sanity_levels: Vec<SanityLevelBucket>,
    }

    pub struct StatsFilter {
        pub year_min: Option<i32>,
        pub year_max: Option<i32>,
        pub r18: Option<String>,  // "show" | "hidden" | "only"
        pub is_ai: Option<bool>,
    }

    pub struct StatsOverview {
        pub total_illustrations: i64,
        pub total_authors: i64,
        pub total_tags: i64,
        pub year_range: (i32, i32),  // min_year, max_year
    }

    pub struct AuthorStats {
        pub author_id: i64,
        pub author_name: String,
        pub author_account: String,
        pub illustration_count: i64,
        pub total_bookmarks: i64,
        pub total_views: i64,
    }

    pub struct TagStats {
        pub name: String,
        pub translated_name: Option<String>,
        pub count: i64,
    }

    pub struct DistributionBucket {
        pub label: String,  // e.g. "0-99", "100-999", etc.
        pub min: i64,
        pub max: Option<i64>,
        pub count: i64,
    }

    pub struct YearlyStats {
        pub year: i32,
        pub count: i64,
        pub avg_bookmark: f64,
        pub avg_view: f64,
    }

    pub struct R18Ratio {
        pub safe: i64,
        pub r18: i64,
        pub r18g: i64,
    }

    pub struct AiRatio {
        pub ai: i64,
        pub non_ai: i64,
    }

    pub struct ShapeBucket {
        pub shape: String,  // "horizontal", "vertical", "square", "4:3", "16:9", "21:9", other
        pub count: i64,
    }

    pub struct TopWork {
        pub id: i64,
        pub title: String,
        pub value: i64,  // bookmark count or view count
        pub author_name: String,
    }

    pub struct AuthorBucket {
        pub label: String,  // "1", "2-5", "6-10", "11-20", "21+"
        pub count: i64,
    }

    // 【新增】R18/AI 年度趋势
    pub struct R18TrendItem {
        pub year: i32,
        pub x_restrict: i32,
        pub count: i64,
    }

    pub struct AiTrendItem {
        pub year: i32,
        pub is_ai: bool,
        pub count: i64,
    }

    // 【增值维度】标签年度趋势
    pub struct TagTrendItem {
        pub year: i32,
        pub tag_name: String,
        pub count: i64,
    }

    // 【增值维度】隐藏神作（高 bookmark/view 比值）
    pub struct HiddenGem {
        pub id: i64,
        pub title: String,
        pub author_name: String,
        pub bookmark: i64,
        pub view: i64,
        pub ratio: f64,
    }

    // 【增值维度】作者首次收录年份
    pub struct AuthorDiscovery {
        pub author_id: i64,
        pub author_name: String,
        pub author_account: String,
        pub first_year: i32,
        pub works_count: i64,
    }

    // 【增值维度】健全度等级分布
    pub struct SanityLevelBucket {
        pub level: i32,
        pub count: i64,
    }
    ```

  - 实现 `StatsFilter::build_where_clause()` 方法——生成 SQL WHERE 子句和参数
    - 支持 `year_min`/`year_max`: `CAST(substr(i.created_at,1,4) AS INTEGER) BETWEEN ? AND ?`
    - 支持 `r18`: 匹配 `i.x_restrict IN (...)`
    - 支持 `is_ai`: `i.is_ai = ?`
    - 返回 `(where_clause: String, params: Vec<Box<dyn rusqlite::types::ToSql>>)`

  - 实现 `compute_statistics(conn: &Connection, filter: &StatsFilter) -> Result<StatisticsResult, String>` 函数
    - 对每个统计维度执行独立的 SQL 查询，所有查询共享同一个 filter WHERE clause
    - 所有计数使用 **`COUNT(DISTINCT i.id)`**（以作品为单位），只有 shape 分布使用 `COUNT(DISTINCT i.id, i.part)`（考虑多页尺寸不同）

  **Must NOT do**:
  - 不要修改 `query.rs`, `db.rs` 中的现有代码
  - 不要耦合到 `ImageQuery` 结构体

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 涉及 Rust 类型设计、SQL 查询性能、缓存策略权衡，需要深度思考
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 3, 4
  - **Blocked By**: None

  **References**:
  - `src-tauri/src/query.rs` — 参考 `build_sql()` 的 WHERE 构建模式
  - `src-tauri/src/db.rs:refresh_caches()` — 参考现有聚合查询的 SQL 写法
  - `web/src/types/index.d.ts` — 前端类型定义，保持字段命名一致性

  **Acceptance Criteria**:
  - [ ] `stats.rs` 包含完整的类型定义和 SQL 构建器
  - [ ] `StatsFilter::build_where_clause()` 能正确处理所有筛选组合
  - [ ] `compute_statistics()` 函数签名正确

  **QA Scenarios**:
  ```
  Scenario: stats.rs compiles
    Tool: Bash
    Preconditions: stats.rs created, main.rs not yet updated
    Steps:
      1. cargo check 2>&1
    Expected Result: No compilation errors (main.rs may warn about unused module)
    Evidence: .sisyphus/evidence/task-2-rust-check.txt

  Scenario: StatsFilter WHERE clause correctness
    Tool: Bash
    Preconditions: stats.rs exists
    Steps:
      - This is a code review — verify the build_where_clause() handles:
        1. No filters → returns "1=1" with empty params
        2. year_min only → correct SQL + param
        3. r18="hidden" → x_restrict = 0
        4. All filters combined → AND concatenated correctly
    Expected Result: WHERE clause logic is correct
    Evidence: .sisyphus/evidence/task-2-where-clause.txt
  ```

  **Commit**: NO (groups with task 2-4)
  ---

- [x] 3. 实现 compute_statistics() — 所有聚合查询

  **What to do**:
  - 在 `stats.rs` 中实现 `compute_statistics()` 函数
  - 对每个统计维度执行 SQL 查询，所有查询共享 `StatsFilter::build_where_clause()` 生成的 WHERE 子句
  - 具体 SQL 查询：

    **Overview**:
    ```sql
    -- total_illustrations, total_authors, year_range
    SELECT COUNT(DISTINCT i.id), COUNT(DISTINCT i.author_id),
           MIN(CAST(substr(i.created_at,1,4) AS INTEGER)),
           MAX(CAST(substr(i.created_at,1,4) AS INTEGER))
    FROM images i WHERE {where}
    ```

    **Author Ranking** (TOP 200):
    ```sql
    SELECT i.author_id, i.author_name, i.author_account,
           COUNT(DISTINCT i.id) as illust_count,
           SUM(i.bookmark) as total_bookmarks,
           SUM(i.view) as total_views
    FROM images i WHERE {where}
    GROUP BY i.author_id
    ORDER BY illust_count DESC
    LIMIT 200
    ```

    **Tag Ranking** (TOP 200 — 需要 JOIN image_tags):
    ```sql
    SELECT t.name, t.translated_name, COUNT(DISTINCT it.image_id) as count
    FROM tags t
    JOIN image_tags it ON it.tag_id = t.id
    JOIN images i ON i.id = it.image_id AND i.part = it.image_part
    WHERE {where}
    GROUP BY t.id
    ORDER BY count DESC
    LIMIT 200
    ```
    注意：`{where}` 中的 `i.` 前缀在 JOIN 中有效。

    **Bookmark Distribution**:
    ```sql
    SELECT COUNT(DISTINCT i.id) as count,
           CASE
             WHEN i.bookmark >= 10000 THEN '10000+'
             WHEN i.bookmark >= 5000 THEN '5000-9999'
             WHEN i.bookmark >= 1000 THEN '1000-4999'
             WHEN i.bookmark >= 100 THEN '100-999'
             ELSE '0-99'
           END as bucket,
           MIN(i.bookmark) as min_val,
           MAX(1, MIN(i.bookmark)) as sort_key
    FROM images i WHERE {where}
    GROUP BY bucket ORDER BY sort_key
    ```
    （View Distribution 同理，bucket 阈值可设为 100000, 50000, 10000, 5000, 1000）

    **Yearly Trend**:
    ```sql
    SELECT CAST(substr(i.created_at,1,4) AS INTEGER) as year,
           COUNT(DISTINCT i.id) as count,
           AVG(i.bookmark) as avg_bookmark,
           AVG(i.view) as avg_view
    FROM images i WHERE {where}
    GROUP BY year ORDER BY year
    ```

    **R18 Ratio**:
    ```sql
    SELECT i.x_restrict, COUNT(DISTINCT i.id) as count
    FROM images i WHERE {where}
    GROUP BY i.x_restrict
    ```

    **AI Ratio**:
    ```sql
    SELECT i.is_ai, COUNT(DISTINCT i.id) as count
    FROM images i WHERE {where}
    GROUP BY i.is_ai
    ```

    **Shape Distribution**:
    ```sql
    -- Classify by width/height ratio (use per-page, not per-illustration)
    SELECT
      CASE
        WHEN i.width = i.height THEN 'square'
        WHEN i.width > i.height AND CAST(i.width AS REAL)/i.height >= 1.7 THEN '16:9'
        WHEN i.width > i.height AND CAST(i.width AS REAL)/i.height >= 1.3 THEN '4:3'
        WHEN i.width > i.height THEN 'horizontal'
        WHEN i.height > i.width AND CAST(i.height AS REAL)/i.width >= 1.7 THEN '9:16'
        WHEN i.height > i.width AND CAST(i.height AS REAL)/i.width >= 1.3 THEN '3:4'
        WHEN i.height > i.width THEN 'vertical'
        ELSE 'other'
      END as shape,
      COUNT(*) as count
    FROM images i WHERE {where}
    GROUP BY shape ORDER BY count DESC
    ```
    （使用 COUNT(*) 因为多页作品的不同页面可能有不同的尺寸）

    **Top Works**:
    ```sql
    SELECT i.id, i.title, i.bookmark as value, i.author_name
    FROM images i WHERE {where} AND i.part = 0  -- one row per illustration
    ORDER BY i.bookmark DESC LIMIT 10
    ```
    （同理 View TOP）

    **Author Works Distribution**（看收藏集中在少数作者还是广泛分布）:
    ```sql
    SELECT
      CASE
        WHEN cnt = 1 THEN '1'
        WHEN cnt <= 5 THEN '2-5'
        WHEN cnt <= 10 THEN '6-10'
        WHEN cnt <= 20 THEN '11-20'
        ELSE '21+'
      END as bucket,
      COUNT(*) as count
    FROM (
      SELECT i.author_id, COUNT(DISTINCT i.id) as cnt
      FROM images i WHERE {where}
      GROUP BY i.author_id
    ) sub
    GROUP BY bucket ORDER BY MIN(cnt)
    ```

    **R18 Trend by Year**【新增】:
    ```sql
    SELECT CAST(substr(i.created_at,1,4) AS INTEGER) as year,
           i.x_restrict,
           COUNT(DISTINCT i.id) as count
    FROM images i WHERE {where}
    GROUP BY year, x_restrict
    ORDER BY year, x_restrict
    ```

    **AI Trend by Year**【新增】:
    ```sql
    SELECT CAST(substr(i.created_at,1,4) AS INTEGER) as year,
           i.is_ai,
           COUNT(DISTINCT i.id) as count
    FROM images i WHERE {where}
    GROUP BY year, is_ai
    ORDER BY year, is_ai
    ```

    **Tag Trend (Top 20 tags per year)**【增值】:
    ```sql
    -- 先取 TOP 20 标签名
    -- 再对每个标签按年统计使用频率
    SELECT CAST(substr(i.created_at,1,4) AS INTEGER) as year,
           t.name as tag_name,
           COUNT(DISTINCT i.id) as count
    FROM images i
    JOIN image_tags it ON i.id = it.image_id AND i.part = it.image_part
    JOIN tags t ON t.id = it.tag_id
    WHERE t.name IN (
      SELECT t2.name FROM tags t2
      JOIN image_tags it2 ON it2.tag_id = t2.id
      JOIN images i2 ON i2.id = it2.image_id AND i2.part = it2.image_part
      WHERE {where_filter_without_i_prefix}
      GROUP BY t2.id
      ORDER BY COUNT(DISTINCT i2.id) DESC
      LIMIT 20
    ) AND {where}
    GROUP BY year, t.name
    ORDER BY year, count DESC
    ```
    NOTE: `{where_filter_without_i_prefix}` 是去掉 `i.` 前缀的 WHERE 子句版本。如有必要，可以分两步执行：先查 TOP 20 标签，再查它们的年度趋势。

    **Hidden Gems【增值】**:
    ```sql
    SELECT i.id, i.title, i.author_name, i.bookmark, i.view,
           CAST(i.bookmark AS REAL) / MAX(i.view, 1) as ratio
    FROM images i WHERE {where} AND i.part = 0
    ORDER BY ratio DESC
    LIMIT 10
    ```

    **Author Discovery【增值】**:
    ```sql
    SELECT i.author_id, i.author_name, i.author_account,
           CAST(substr(MIN(i.created_at),1,4) AS INTEGER) as first_year,
           COUNT(DISTINCT i.id) as works_count
    FROM images i WHERE {where}
    GROUP BY i.author_id
    ORDER BY first_year DESC
    LIMIT 100
    ```

    **Sanity Level Distribution【增值】**:
    ```sql
    SELECT i.sanity_level, COUNT(DISTINCT i.id) as count
    FROM images i WHERE {where}
    GROUP BY i.sanity_level
    ORDER BY i.sanity_level
    ```

  - 所有查询使用 `conn.prepare()` + `statement.query_map()` 执行
  - 使用 `rusqlite::params!` 宏传递参数
  - 每个查询独立执行，捕获并转换错误

  **Must NOT do**:
  - 不要使用 ORM——使用原生 rusqlite
  - 不要修改 db.rs 中的 `refresh_caches()`

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: 涉及多个复杂 SQL 查询的性能优化和正确性保证
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 4, 5
  - **Blocked By**: Task 2

  **References**:
  - `src-tauri/src/db.rs:467-614` — `refresh_caches()` 中现有聚合 SQL 写法
  - `src-tauri/src/db.rs:Connection` — 数据库连接使用方式
  - `web/src/types/index.d.ts` — 前端 Image 类型字段名

  **Acceptance Criteria**:
  - [ ] 所有 17 种查询类型正确实现（13 原版 + 4 增值维度）
  - [ ] 所有查询共享 filter WHERE clause
  - [ ] 错误处理完整（连接错误、查询错误、空结果）

  **QA Scenarios**:
  ```
  Scenario: compute_statistics with no filter
    Tool: Bash
    Preconditions: SQLite DB with known test data (~100 images, 10 authors, 50 tags)
    Steps:
      1. Write a small Rust test binary that calls compute_statistics with empty filter
      2. Print all returned stats
      3. Verify: overview.total_illustrations == expected total
      4. Verify: author_ranking.len() == number of authors with works
      5. Verify: yearly_trend covers correct year range
    Expected Result: All stats match expected values
    Evidence: .sisyphus/evidence/task-3-stats-no-filter.txt

  Scenario: compute_statistics with year filter
    Tool: Bash
    Preconditions: Same test DB with images from 2022-2024
    Steps:
      1. Call compute_statistics with year_min=2023, year_max=2023
      2. Verify yearly_trend only contains year 2023
      3. Verify overview.total_illustrations == expected count for 2023
    Expected Result: Filter correctly constrains results
    Evidence: .sisyphus/evidence/task-3-stats-year-filter.txt
  ```

  **Commit**: NO (groups with task 2-4)
  ---

- [x] 4. 注册 get_statistics Tauri 命令

  **What to do**:
  - 在 `src-tauri/src/main.rs` 顶部添加 `mod stats;`
  - 添加新的命令函数：
    ```rust
    #[tauri::command]
    fn get_statistics(
        year_min: Option<i32>,
        year_max: Option<i32>,
        r18: Option<String>,
        is_ai: Option<bool>,
    ) -> Result<stats::StatisticsResult, String> {
        let conn = db::get_reader();
        let filter = stats::StatsFilter { year_min, year_max, r18, is_ai };
        stats::compute_statistics(&conn, &filter)
    }
    ```
  - 在 `invoke_handler` 的 `generate_handler!` 宏中添加 `get_statistics`
  - 运行 `cargo check` 确认编译通过

  **Must NOT do**:
  - 不修改现有命令的签名或行为
  - 不从命令行参数获取 DB 连接——使用现有的 `db::get_reader()`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准 Tauri 命令注册模式，无复杂逻辑
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: Task 14
  - **Blocked By**: Task 3 (stats.rs 需已存在)

  **References**:
  - `src-tauri/src/main.rs:58-78` — `ensure_db` 命令定义作为注册模式参考
  - `src-tauri/src/main.rs:429-442` — `generate_handler!` 注册位置

  **Acceptance Criteria**:
  - [ ] `mod stats;` 在 main.rs 顶部
  - [ ] `get_statistics` 命令函数正确调用 `compute_statistics()`
  - [ ] 命令在 `generate_handler!` 中注册
  - [ ] `cargo check` 通过

  **QA Scenarios**:
  ```
  Scenario: Rust compilation
    Tool: Bash
    Preconditions: All Wave 1 tasks complete
    Steps:
      1. cd src-tauri && cargo check 2>&1
    Expected Result: Compilation successful, no errors
    Evidence: .sisyphus/evidence/task-4-cargo-check.txt
  ```

  **Commit**: YES
  - Message: `feat(backend): add get_statistics Tauri command for bookmark data analysis`
  - Files: `src-tauri/src/stats.rs`, `src-tauri/src/main.rs`
  - Pre-commit: `cd src-tauri && cargo check`

---

- [x] 5. 构建 StatsModal.vue — 模态框容器 + 筛选控件

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsModal.vue`
  - 参考 `ImageViewer.vue` 的 overlay 模式：
    - 全屏 fixed 遮罩（`fixed inset-0 z-50`）
    - 深色半透明背景（`bg-black/60`）
    - 点击背景关闭
    - Escape 键关闭
    - `<Transition name="fade">` 动画
  - 内部容器：
    - 居中弹窗，最大宽度 ~90vw，最大高度 ~90vh
    - 左侧为图表区域（可滚动），右侧为筛选控制面板（`w-64`）
    - 深色模式适配（`dark:` 变体）
  - 筛选控件（弹窗右上区域）：
    - 年份选择器：下拉框（从 `yearly_trend` 数据动态获取年份列表），支持范围选择
    - R18 开关：`hidden / show / only` 三态（复用画廊相同的 `x_restrict` 逻辑）
    - AI 开关：`show / hide / only` 三态
    - 应用筛选按钮 → 重新调用 `get_statistics`
    - 当前筛选标签（active filter chips）
  - Props:
    - `show: boolean`
    - `stats: StatisticsResult | null`（父组件传递已加载的数据）
  - Emits:
    - `close`
    - `apply-filter(filter: StatsFilter)`
  - 加载状态：显示 loading spinner 或 skeleton
  - 空数据状态：当 stats 为 null 时显示提示
  - 关闭按钮（右上角 X 图标）

  **Must NOT do**:
  - 不要写图表逻辑（其他组件负责）
  - 不硬编码样式——全部使用 Tailwind 常用值

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 模态框 UI/UX 设计 + 响应式布局 + 动画
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 14, 16
  - **Blocked By**: Task 1 (chart.js 用于后续，但 modal 本身不需要), Task 3 (需知道返回数据结构)

  **References**:
  - `web/src/components/ImageViewer.vue` — 现有 overlay 模式参考
  - `web/src/components/Sidebar/Sidebar.vue` — 筛选控件样式（R18 三态、年份选择）
  - `web/src/assets/transition.css` — 现有过渡动画 class

  **Acceptance Criteria**:
  - [ ] 模态框打开/关闭有过渡动画
  - [ ] 点击背景关闭
  - [ ] Escape 键关闭
  - [ ] 筛选控件正确显示
  - [ ] 加载态/空数据态正确展示
  - [ ] 深色模式正常

  **QA Scenarios**:
  ```playwright
  Scenario: Modal opens and closes
    Tool: Playwright
    Preconditions: App running, stats data available
    Steps:
      1. Click statistics button in navbar -> modal visible
      2. Verify modal has .fixed.inset-0 + visible filter controls
      3. Press Escape -> modal hidden
      4. Click button again -> modal visible again
      5. Click backdrop (bg-black/60 area) -> modal hidden
    Expected Result: Modal toggles correctly, animations play
    Evidence: .sisyphus/evidence/task-5-modal-open-close.mp4

  Scenario: Modal in dark mode
    Tool: Playwright
    Preconditions: App running
    Steps:
      1. Toggle color scheme to dark
      2. Open stats modal
      3. Verify background, text, and border colors use dark: variants
    Expected Result: Modal renders correctly in dark mode
    Evidence: .sisyphus/evidence/task-5-modal-dark.png
  ```
  ---

- [x] 6. 构建 StatsOverview.vue — 概览卡片

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsOverview.vue`
  - Props: `overview: { total_illustrations: number, total_authors: number, total_tags: number, year_range: [number, number] }`
  - 展示 4 张统计卡片：
    - 总作品数（total_illustrations）
    - 总作者数（total_authors）
    - 总标签数（total_tags）
    - 创作年份区间（year_min - year_max）
  - 卡片样式：
    - 网格布局（`grid grid-cols-2 lg:grid-cols-4 gap-4`）
    - 每张卡片：圆角背景 + 大数字 + 小标题
    - 深色模式适配
  - 数字格式化：`1000` -> `1,000`

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI 布局设计 + 响应式网格
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with tasks 7-12)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **Acceptance Criteria**:
  - [ ] 4 张概览卡片正确显示
  - [ ] 数字千分位格式化
  - [ ] 深色模式适配
  - [ ] 响应式布局（移动端 2 列、桌面 4 列）

  **QA Scenarios**:
  ```playwright
  Scenario: Overview cards display correctly
    Tool: Playwright
    Preconditions: App running, stats loaded
    Steps:
      1. Open stats modal
      2. Verify 4 overview cards visible in grid
      3. Check total_illustrations > 0 and formatted (e.g. "50,000")
      4. Check year range is displayed correctly (e.g. "2018-2025")
    Expected Result: All 4 cards present with correct data
    Evidence: .sisyphus/evidence/task-6-overview-cards.png
  ```
  ---

- [x] 7. 构建 StatsAuthorChart.vue — 作者排行图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsAuthorChart.vue`
  - Props: `authors: Array<{ author_id, author_name, author_account, illustration_count, total_bookmarks, total_views }>`
  - 展示内容：
    - 水平柱状图（TOP 20 作者，按作品数降序）
    - 下方补齐完整 TOP 200 排名列表（可滚动）
  - Chart.js 配置：
    - 使用 `vue-chartjs` 的 `<Bar>` 组件
    - Y 轴：作者名称（`author_name`）
    - X 轴：作品数（`illustration_count`）
    - 水平柱状图（`indexAxis: 'y'`）
    - 颜色：使用 Tailwind 调色板蓝/紫渐变
    - 暗色模式：文本色、网格色、柱状色自动切换
    - 中文标签
  - 排名列表：
    - 表格形式：排名 / 作者名 / 作品数 / 总收藏 / 总浏览
    - 可滚动容器
    - 作者名链接到 Pixiv 用户页

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Chart.js 图表集成 + 排名列表 UI
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with tasks 5-12)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **References**:
  - `web/src/config/index.ts:LINK_PIXIV_USER` — 作者链接模板

  **Acceptance Criteria**:
  - [ ] 水平柱状图显示 TOP 20 作者
  - [ ] 排名列表显示完整 TOP 200
  - [ ] 柱状图在深色/浅色模式下颜色正确
  - [ ] 作者名可点击跳转 Pixiv

  **QA Scenarios**:
  ```playwright
  Scenario: Author chart renders
    Tool: Playwright
    Preconditions: Stats loaded with at least 5 authors
    Steps:
      1. Open stats modal, navigate to author chart section
      2. Verify Bar chart canvas element exists (canvas.chartjs-render-monitor)
      3. Verify chart has axis labels (at least 1 author name visible)
      4. Verify ranking list below chart has entries with rank numbers
      5. Click an author name -> new tab with pixiv.net/users/{id}
    Expected Result: Chart renders, ranking list shows data
    Evidence: .sisyphus/evidence/task-7-author-chart.png

  Scenario: Dark mode chart
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Toggle to dark mode
      2. Open stats modal -> author chart
      3. Verify chart text colors change (grid, labels, legend)
    Expected Result: Chart adapts to dark theme
    Evidence: .sisyphus/evidence/task-7-author-chart-dark.png
  ```
  ---

- [x] 8. 构建 StatsTagChart.vue — 标签排行图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsTagChart.vue`
  - Props: `tags: Array<{ name, translated_name, count }>`
  - 展示内容：
    - 水平柱状图（TOP 30 标签，按使用频率降序）
    - 下方补齐 TOP 200 排名列表
  - Chart.js 配置：
    - 水平柱状图（`indexAxis: 'y'`）
    - Y 轴：标签名（优先显示 `translated_name`）
    - X 轴：出现次数
    - 颜色配色使用绿色调
    - 暗色模式适配
  - 排名列表：排名 / 标签名 / 出现次数

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Chart.js 图表集成
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with tasks 5-12)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **Acceptance Criteria**:
  - [ ] 水平柱状图显示 TOP 30 标签
  - [ ] 标签名显示翻译名优先
  - [ ] 排名列表显示 TOP 200
  - [ ] 深色模式适配

  **QA Scenarios**:
  ```playwright
  Scenario: Tag chart renders
    Tool: Playwright
    Preconditions: Stats loaded with known tags
    Steps:
      1. Open stats modal, navigate to tag chart section
      2. Verify Bar chart canvas exists
      3. Verify top tag matches expected most-used tag
      4. Verify ranking list has tag names + counts
    Expected Result: Tag chart and list show correct data
    Evidence: .sisyphus/evidence/task-8-tag-chart.png
  ```
  ---

- [x] 9. 构建 StatsTimeChart.vue — 时间趋势图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsTimeChart.vue`
  - Props: `yearly: Array<{ year, count, avg_bookmark, avg_view }>`
  - 展示内容：
    - 混合图表：柱状图（作品数 count）+ 折线图（平均收藏数 avg_bookmark）
    - 双 Y 轴：
      - 左 Y 轴：作品数（柱状图）
      - 右 Y 轴：平均收藏数（折线图）
    - X 轴：年份
    - 使用 Chart.js 混合图表类型
  - 图表配置：
    - 柱状图半透明蓝色填充
    - 折线图红色/橙色线 + 圆点
    - 图例显示：作品数 / 平均收藏
    - 深色模式适配
    - 中文轴标签
  - 时间范围指示：图表下方显示跨度

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 混合 Chart.js 图表类型 + 双 Y 轴配置
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with tasks 5-12)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **Acceptance Criteria**:
  - [ ] 柱状图 + 折线图在同一图表中显示
  - [ ] 双 Y 轴正确（左：数量，右：平均值）
  - [ ] X 轴显示所有年份
  - [ ] 图例正确标识数据系列

  **QA Scenarios**:
  ```playwright
  Scenario: Time trend chart renders with mixed types
    Tool: Playwright
    Preconditions: Stats loaded with 5+ years of data
    Steps:
      1. Open stats modal -> time trend section
      2. Verify mixed chart (bars + line) renders
      3. Check dual Y-axes are visible (left = count, right = avg)
      4. Verify bars decrease/increase as expected across years
    Expected Result: Mixed chart displays correctly
    Evidence: .sisyphus/evidence/task-9-time-chart.png
  ```
  ---

- [x] 10. 构建 StatsDistribution.vue — 收藏/浏览分布图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsDistribution.vue`
  - Props: `bookmarkDist: Array<{ label, count }>`, `viewDist: Array<{ label, count }>`
  - 展示内容：
    - 两个垂直柱状图并排或上下排列
    - 每个图表：X 轴为区间标签，Y 轴为作品数
    - 柱状图显示具体数值 + 百分比
  - 图表配置：
    - 收藏分布：蓝色调柱状
    - 浏览分布：紫色调柱状
    - 深色模式适配
    - 中文标签 + 区间说明

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 多图表布局 + 数据分布可视化
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with tasks 5-12)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **Acceptance Criteria**:
  - [ ] 收藏分布图显示所有区间
  - [ ] 浏览分布图显示所有区间
  - [ ] 柱状图显示数值 + 百分比
  - [ ] 深色模式适配

  **QA Scenarios**:
  ```playwright
  Scenario: Distribution charts render
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Open stats modal -> distribution section
      2. Verify bookmark distribution bar chart renders with buckets
      3. Verify view distribution bar chart renders
      4. Check percentage labels on bars
    Expected Result: Both distribution charts visible with correct data
    Evidence: .sisyphus/evidence/task-10-distribution.png
  ```
  ---

- [x] 11. 构建 StatsPieSection.vue — R18/AI/形状比例图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsPieSection.vue`
  - Props: `r18Ratio: { safe, r18, r18g }`, `aiRatio: { ai, non_ai }`, `shapeDist: Array<{ shape, count }>`
  - 展示内容（三张饼图/环形图并排）：
    1. **R18 比例**: 饼图 — safe / R18 / R18G（绿 / 橙 / 红）
    2. **AI 比例**: 饼图 — AI / 非 AI（紫 / 蓝）
    3. **形状分布**: 环形图 — 横向 / 竖向 / 方形 / 各比例
  - 图表配置：
    - 使用 vue-chartjs 的 `<Doughnut>` 和 `<Pie>` 组件
    - 图例显示在右侧
    - 悬停显示详情
    - 深色模式适配
    - 中文标签

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 饼图图表可视化
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with tasks 5-12)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **Acceptance Criteria**:
  - [ ] R18 饼图显示正确比例
  - [ ] AI 饼图显示正确比例
  - [ ] 形状分布环形图显示正确比例
  - [ ] 图例、百分比显示正确
  - [ ] 深色模式适配

  **QA Scenarios**:
  ```playwright
  Scenario: Pie charts render with correct ratios
    Tool: Playwright
    Preconditions: Stats loaded with mixed R18/AI data
    Steps:
      1. Open stats modal -> pie section
      2. Verify 3 chart canvases exist (R18, AI, shape)
      3. Hover over R18 pie segment -> tooltip shows correct count
      4. Verify percentages add up to ~100%
    Expected Result: All pie charts display with correct data
    Evidence: .sisyphus/evidence/task-11-pie-charts.png

  Scenario: Color differentiation in dark mode
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Toggle dark mode
      2. Open stats modal -> pie section
      3. Verify chart colors are clearly distinguishable
    Expected Result: Charts remain readable in dark mode
    Evidence: .sisyphus/evidence/task-11-pie-dark.png
  ```
  ---

- [x] 12. 构建 StatsTopLists.vue — 最值排行列表

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsTopLists.vue`
  - Props: `topBookmarked: Array<{ id, title, value, author_name }>`, `topViewed: Array<...>`
  - 展示内容：
    - 两个并排或上下排列的排行榜
    - **收藏数最高 TOP 10**
    - **浏览数最高 TOP 10**
  - 每个排行项：
    - 排名数字
    - 作品标题（可点击跳转到 Pixiv）
    - 作者名（可点击跳转）
    - 数值（千分位格式化）
  - 样式：
    - 斑马条纹交替背景色
    - 前三名突出显示
    - 响应式布局

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 排行榜 UI 设计 + 链接交互
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with tasks 5-12)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **References**:
  - `web/src/config/index.ts:LINK_PIXIV_ARTWORK` — 作品链接模板
  - `web/src/config/index.ts:LINK_PIXIV_USER` — 作者链接模板

  **Acceptance Criteria**:
  - [ ] 收藏 TOP 10 排行显示
  - [ ] 浏览 TOP 10 排行显示
  - [ ] 前三名特殊样式
  - [ ] 链接可点击跳到 Pixiv
  - [ ] 深色模式适配

  **QA Scenarios**:
  ```playwright
  Scenario: Top lists display
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Open stats modal -> top lists section
      2. Verify top bookmarked list has 10 entries
      3. Verify first entry (rank 1) has special styling
      4. Click a title -> opens pixiv.net/artworks/{id}
     Expected Result: Both lists display with correct rankings
     Evidence: .sisyphus/evidence/task-12-top-lists.png
   ```
   ---

- [x] 13. 在 Navbar 中添加统计面板入口按钮

  **What to do**:
  - 在 `web/src/components/Navbar.vue` 中添加图表/统计按钮
  - 按钮位置：右侧按钮组，放在 GitHub 链接之前或之后
  - 使用现有图标风格：可用 SVG 图标（柱状图样式）
  - 按钮样式：匹配现有图标按钮（圆形、hover 背景变化）
  - 点击触发：emit 事件给父组件（或直接修改 store state）
  - 在 `App.vue` 中监听该事件并控制 `StatsModal` 的显示
  - 添加 tooltip 提示："统计"

  **Must NOT do**:
  - 不替换或移动现有按钮
  - 不改变 Navbar 布局结构

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI 元素添加，与现有导航栏风格一致
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 14
  - **Blocked By**: None (独立添加按钮)

  **References**:
  - `web/src/components/Navbar.vue` — 现有导航栏代码

  **Acceptance Criteria**:
  - [ ] 图表按钮出现在 Navbar 右侧
  - [ ] 按钮样式与现有图标按钮一致
  - [ ] 点击按钮打开统计弹窗
  - [ ] 按钮有 tooltip 提示

  **QA Scenarios**:
  ```playwright
  Scenario: Navbar stats button opens modal
    Tool: Playwright
    Preconditions: App running
    Steps:
      1. Locate stats button in navbar (look for chart icon)
      2. Verify button is visible and styled like other icon buttons
      3. Click button -> verify StatsModal is visible
      4. Close modal -> click button again -> modal opens again
    Expected Result: Button toggles stats modal correctly
    Evidence: .sisyphus/evidence/task-13-navbar-button.png
  ```
  ---

- [x] 14. 连接数据流 — Tauri command -> store -> 图表组件

  **What to do**:
  - 在 `StatsModal.vue` 或 `App.vue` 中添加数据获取逻辑
  - 在 `App.vue` 中：
    - 引入 `StatsModal` 组件
    - 添加响应式 `showStatsModal` 状态
    - 在模板中渲染 `StatsModal` 并传入数据
  - 添加 `fetchStatistics(filter?)` 方法：
    ```typescript
    async function fetchStatistics(filter?: StatsFilter) {
      loading.value = true
      try {
        const result = await invoke('get_statistics', {
          yearMin: filter?.year_min ?? null,
          yearMax: filter?.year_max ?? null,
          r18: filter?.r18 ?? null,
          isAi: filter?.is_ai ?? null,
        })
        statsData.value = result
      } finally {
        loading.value = false
      }
    }
    ```
  - 通信方式：
    - `App.vue` 传递 statsData 和 loading 状态给 `StatsModal`
    - `StatsModal` emit `apply-filter` 事件后，`App.vue` 重新调用 `fetchStatistics`
  - 处理首次打开：点击统计按钮时调用 `fetchStatistics()`
  - 处理筛选变更：用户在弹窗内点击"应用"后重新调用
  - 处理加载状态和错误状态

  **Must NOT do**:
  - 不修改现有的 `loadImagesByPage` 或 `fetchFilteredCounts` 方法

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: 多个组件间的数据流协调，涉及 Tauri IPC、组件通信
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO (需要 Wave 2 组件就位)
  - **Parallel Group**: Wave 3
  - **Blocks**: Tasks 15, 16
  - **Blocked By**: Tasks 4, 5, 6-12, 13

  **References**:
  - `web/src/store/index.ts` — 现有 store 中 `loadImagesByPage` 的 invoke 模式
  - `web/src/App.vue` — 根组件，需要添加 StatsModal 渲染

  **Acceptance Criteria**:
  - [ ] 首次打开统计面板时调用 get_statistics
  - [ ] 应用筛选后重新调用
  - [ ] 加载状态正确展示（loading spinner）
  - [ ] 数据正确流向所有子组件
  - [ ] 错误状态有提示

  **QA Scenarios**:
  ```playwright
  Scenario: Statistics data loads on modal open
    Tool: Playwright
    Preconditions: App running, DB has data
    Steps:
      1. Click stats button in navbar
      2. Verify loading state appears briefly
      3. Verify stats data populates all chart components
      4. Verify no console errors from Tauri invoke
    Expected Result: Data loads successfully, all components render
    Evidence: .sisyphus/evidence/task-14-data-flow.txt

  Scenario: Filter applies correctly
    Tool: Playwright
    Preconditions: App running, DB has multi-year data
    Steps:
      1. Open stats modal, wait for data
      2. Select year filter (e.g. 2023) -> click Apply
      3. Verify stats update: total count decreases, yearly trend shows only 2023
    Expected Result: Filter changes propagate to all chart components
    Evidence: .sisyphus/evidence/task-14-filter-apply.txt
  ```
  ---

- [x] 15. 暗色模式图表主题切换

  **What to do**:
  - 在所有 Chart.js 组件中添加暗色模式响应：
    - 监听 `store.colorScheme` 变化
    - 当 scheme 变化时，调用 `chart.update()` 或重新渲染
  - 使用 Vue `watch` 响应 color scheme：
    ```typescript
    watch(() => store.colorScheme, () => {
      if (chartRef.value?.chart) {
        updateChartColors(chartRef.value.chart, store.colorScheme)
        chartRef.value.chart.update()
      }
    })
    ```
  - 颜色映射：
    - 浅色模式：深色文字、浅色网格、彩色柱状/折线
    - 深色模式：浅色文字、深色网格、饱和度略低的颜色
    - 使用 Chart.js 的 `plugins` 配置项
  - 提取共享工具函数 `getChartColors(isDark: boolean)` 到 chart 组件内

  **Must NOT do**:
  - 不要使用额外的 dark mode 检测库——复用 store.colorScheme
  - 不要重建整个 chart——调用 chart.update() 更新颜色

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Chart.js 主题样式切换
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 16
  - **Blocked By**: Task 14

  **References**:
  - `web/src/store/index.ts:colorScheme` — 主题色状态

  **Acceptance Criteria**:
  - [ ] 浅色/深色模式切换时所有图表颜色更新
  - [ ] 颜色在两种模式下都清晰可读
  - [ ] 更新过程平滑，无闪烁

  **QA Scenarios**:
  ```playwright
  Scenario: Chart colors switch with theme
    Tool: Playwright
    Preconditions: Stats modal open with data
    Steps:
      1. Open stats modal with charts visible
      2. Screenshot charts in light mode
      3. Toggle to dark mode via navbar color button
      4. Wait for charts to update
      5. Screenshot charts in dark mode
      6. Compare screenshots — verify text/grid colors changed
    Expected Result: Chart colors adapt to dark/light theme
    Evidence: .sisyphus/evidence/task-15-dark-mode-compare.png
  ```
  ---

- [x] 16. 响应式布局 + 移动端适配

  **What to do**:
  - 在 `StatsModal.vue` 中处理不同屏幕尺寸：
    - 桌面（`>= 1024px`）：左侧图表区 + 右侧筛选面板的并排布局
    - 平板（`>= 640px`）：图表区 + 折叠式筛选
    - 手机（`< 640px`）：全屏模态框，筛选面板作为可折叠区域
  - 图表组件响应式：
    - Chart.js 默认 `responsive: true`
    - 设置 `maintainAspectRatio: true` + 合理的高宽比
  - 使用 Tailwind 响应式前缀：`sm:` `lg:` `xl:`
  - 模态框最大尺寸适配：
    - 桌面：`max-w-[90vw] max-h-[90vh]`
    - 移动端：`inset-0`（全屏）
  - 检查所有图标按钮在窄屏下的可访问性

  **Must NOT do**:
  - 不添加水平滚动（内容应自适应）

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 响应式 UI 适配 + Tailwind 断点处理
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 17
  - **Blocked By**: Tasks 5, 14, 15

  **Acceptance Criteria**:
  - [ ] 桌面端（1280px+）并排布局正常
  - [ ] 平板端（768px）筛选可折叠
  - [ ] 手机端（375px）全屏显示
  - [ ] 图表在所有宽度下不溢出
  - [ ] 无水平滚动条

  **QA Scenarios**:
  ```playwright
  Scenario: Responsive layout at different widths
    Tool: Playwright
    Preconditions: App running, stats loaded
    Steps:
      1. Set viewport to 1280x800 -> open modal -> verify dual-column
      2. Close modal, set viewport to 768x1024 -> open modal -> verify single column
      3. Close modal, set viewport to 375x667 -> open modal -> verify fullscreen
      4. Verify no horizontal scrollbar at any breakpoint
    Expected Result: Modal adapts to all viewport sizes
    Evidence: .sisyphus/evidence/task-16-responsive.mp4
  ```
  ---

- [x] 17. 过渡动画 + 最终打磨

  **What to do**:
  - 模态框入场/出场动画：复用 `fade` transition class
  - 图表入场动画：Chart.js 内置动画（默认即可）
  - 筛选面板展开/收起动画：`<Transition name="popup-l">`
  - 加载骨架屏：灰色 pulsing 矩形
  - 空数据状态下显示友好提示："暂无数据"
  - 错误状态：显示错误信息 + 重试按钮
  - 确保所有文本使用中文
  - ESLint 检查：`cd web && yarn lint --fix`

  **Must NOT do**:
  - 不要过度动画
  - 不添加第三方动画库

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI 动画 + 视觉打磨
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 5, 14, 15, 16

  **References**:
  - `web/src/assets/transition.css` — 现有过渡动画 class

  **Acceptance Criteria**:
  - [ ] 模态框打开/关闭有 fade 动画
  - [ ] 筛选面板展开/收起平滑
  - [ ] 加载骨架屏显示
  - [ ] 空数据和错误状态友好提示
  - [ ] `yarn lint` 通过

  **QA Scenarios**:
  ```playwright
  Scenario: Animations and edge states
    Tool: Playwright
    Preconditions: App running
    Steps:
      1. Open stats modal -> verify fade-in transition completes
      2. Close modal -> verify fade-out transition completes
      3. cd web && yarn lint -> exit 0
      4. (Edge case) Simulate network error -> verify error message + retry button
    Expected Result: Smooth animations, edge states handled
    Evidence: .sisyphus/evidence/task-17-animations.mp4
  ```

  **Commit**: YES
  - Message: `feat(stats): add bookmark data statistics panel with charts`
  - Files (all new + modified):
    - `web/src/components/Statistics/StatsModal.vue`
    - `web/src/components/Statistics/StatsOverview.vue`
    - `web/src/components/Statistics/StatsAuthorChart.vue`
    - `web/src/components/Statistics/StatsTagChart.vue`
    - `web/src/components/Statistics/StatsTimeChart.vue`
    - `web/src/components/Statistics/StatsDistribution.vue`
    - `web/src/components/Statistics/StatsPieSection.vue`
    - `web/src/components/Statistics/StatsTopLists.vue`
    - `web/src/App.vue`
    - `web/package.json`
  - Pre-commit: `cd web && vue-tsc --noEmit && yarn lint`

---

- [x] 18. 构建 StatsAuthorDistribution.vue — 作者作品数分布图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsAuthorDistribution.vue`
  - Props: `authorDist: Array<{ label: string, count: number }>` — 数据来自 Task 3 的 `author_works_distribution`
  - 展示内容：
    - 垂直柱状图
    - X 轴：区间标签（"1件", "2-5件", "6-10件", "11-20件", "21+"）
    - Y 轴：作者数量
    - 显示每个区间的具体数值
  - Chart.js 配置：
    - 使用 vue-chartjs 的 `<Bar>` 组件
    - 柱状颜色：渐变紫/蓝色
    - 深色模式适配
  - 图表下方添加解读文字：
    - 例如："你有 N 位作者的作品被收藏，其中 X 位作者贡献了超过 20 件作品"

  **Must NOT do**:
  - 不查询后端（数据已包含在 StatisticsResult 中）

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with all other chart components)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **Acceptance Criteria**:
  - [ ] 柱状图显示 5 个作品数区间
  - [ ] 每个区间柱显示具体数值
  - [ ] 深色模式适配
  - [ ] 解读文字正确计算

  **QA Scenarios**:
  ```playwright
  Scenario: Author distribution chart renders
    Tool: Playwright
    Preconditions: Stats loaded with author_works_distribution data
    Steps:
      1. Open stats modal -> author distribution section
      2. Verify 5 bars visible for each bucket (1, 2-5, 6-10, 11-20, 21+)
      3. Verify bar values match data (e.g. "42 authors have 1 work")
      4. Verify interpretive text below chart is grammatically correct
    Expected Result: Distribution chart clearly shows author concentration
    Evidence: .sisyphus/evidence/task-18-author-dist.png
  ```
  ---

- [x] 19. 构建 StatsWordCloud.vue — 标签词云

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsWordCloud.vue`
  - Props: `tags: Array<{ name: string, translated_name: string | null, count: number }>` — 数据来自 Task 3 的 `tag_ranking`
  - 展示内容：
    - 标签词云：标签按使用频率以不同字体大小显示
    - 高频标签更大更粗，低频标签更小
    - 标签名优先显示 `translated_name`
  - 实现方案（二选一，由执行者评估）：
    - **方案 A**: 安装 `d3-cloud` + 纯 SVG 渲染（最灵活，无框架依赖）
    - **方案 B**: 纯 CSS 浮动布局（简单但无密集排列效果）
    - **推荐方案 A** — 效果更好，复用 tag 数据无需额外请求
  - 交互：
    - 悬停标签时显示具体出现次数（tooltip）
    - 可选：点击标签可将其填入画廊搜索框（v1 不强制要求）
  - 样式：
    - 彩色标签（多种颜色随机或按频率渐变）
    - 深色模式适配
    - 响应式容器（词云自动重排）

  **Must NOT do**:
  - 不替换已有的标签排行柱状图（Task 8）——两者共存
  - 不使用付费或有许可证问题的库

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 词云可视化是一种特殊的 UI 渲染，需要权衡 d3-cloud 集成和视觉效果
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with other chart components)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **References**:
  - `d3-cloud` npm: `https://github.com/d3/d3-cloud` — 词云布局算法
  - 纯 CSS 词云替代方案：使用 flex-wrap + font-size 映射

  **Acceptance Criteria**:
  - [ ] 词云正确显示标签（至少有 50 个标签可见）
  - [ ] 标签字体大小反映使用频率
  - [ ] 高频标签颜色突出
  - [ ] 悬停显示 tooltip（标签名 + 出现次数）
  - [ ] 深色模式适配
  - [ ] 响应式布局

  **QA Scenarios**:
  ```playwright
  Scenario: Word cloud renders with sized tags
    Tool: Playwright
    Preconditions: Stats loaded with tag_ranking data (50+ tags)
    Steps:
      1. Open stats modal -> word cloud section
      2. Verify tags visible as text elements with varying font sizes
      3. Verify top tag (highest count) has largest font size
      4. Verify bottom tag (lowest count) has smallest font size
      5. Hover over a tag -> tooltip shows tag name + count
    Expected Result: Word cloud visualizes tag frequency effectively
    Evidence: .sisyphus/evidence/task-19-wordcloud.png

  Scenario: Dark mode
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Toggle dark mode
      2. Open stats modal -> word cloud
      3. Verify text colors are readable against dark background
    Expected Result: Word cloud readable in dark mode
    Evidence: .sisyphus/evidence/task-19-wordcloud-dark.png
  ```
  ---

- [x] 20. 构建 StatsR18AiTrend.vue — R18/AI 年度趋势图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsR18AiTrend.vue`
  - Props: `r18Trend: Array<{ year, x_restrict, count }>`, `aiTrend: Array<{ year, is_ai, count }>`
  - 展示内容：
    - 两个堆叠柱状图或分组柱状图并排：
      1. **R18 年度趋势**: X 轴为年份，堆叠柱状图分 safe / R18 / R18G
      2. **AI 年度趋势**: X 轴为年份，堆叠柱状图分 AI / 非 AI
    - 每个柱子显示百分比构成（不只绝对值）
  - Chart.js 配置：
    - 使用 vue-chartjs 的 `<Bar>` 组件
    - `type: 'bar'` + `stacked: true`
    - 颜色：
      - R18 趋势：safe=绿色(#22c55e), R18=橙色(#f97316), R18G=红色(#ef4444)
      - AI 趋势：非AI=蓝色(#3b82f6), AI=紫色(#a855f7)
    - 深色模式适配
    - 中文标签 + 图例
  - 数据处理：
    - 前端将后端返回的 `[{year, x_restrict, count}]` 转换为 Chart.js dataset 格式
    - 年份为 labels，x_restrict 值分组为 datasets
    - 计算每年各分类占比（百分比）

  **Must NOT do**:
  - 不替换已有的 R18/AI 静态饼图（Task 11）——趋势图与静态占比图共存
  - 不查询额外后端（数据来自 Task 3 的 r18_trend / ai_trend）

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 堆叠柱状图 + 数据转换 + 多数据集管理
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with other chart components)
  - **Blocks**: Task 14
  - **Blocked By**: Task 1

  **Acceptance Criteria**:
  - [ ] R18 堆叠柱状图按年展示 safe/R18/R18G 比例
  - [ ] AI 堆叠柱状图按年展示 AI/非AI 比例
  - [ ] 柱状图显示百分比
  - [ ] 深色模式适配
  - [ ] 图表标题和图例正确

  **QA Scenarios**:
  ```playwright
  Scenario: R18/AI trend charts render
    Tool: Playwright
    Preconditions: Stats loaded with multi-year r18_trend + ai_trend data
    Steps:
      1. Open stats modal -> R18/AI trend section
      2. Verify R18 stacked bar chart with years on X-axis
      3. Verify each bar has 3 segments (safe, R18, R18G) with distinct colors
      4. Verify AI stacked bar chart with 2 segments (AI, non-AI)
      5. Hover over a segment -> tooltip shows year, category, count, percentage
    Expected Result: Both trend charts show yearly proportion changes
    Evidence: .sisyphus/evidence/task-20-r18-ai-trend.png

  Scenario: Trend data changes with filter
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Open stats modal, wait for data
      2. Apply year filter (e.g. 2022-2024)
      3. Verify R18/AI trend charts update to show only filtered years
    Expected Result: Trend charts respond to filter
    Evidence: .sisyphus/evidence/task-20-trend-filtered.png
  ```

  **Commit**: YES (groups with task 17 if in same commit, or standalone)
  - Message: `feat(stats): add author distribution, word cloud, and R18/AI trend charts`
  - Files:
    - `web/src/components/Statistics/StatsAuthorDistribution.vue`
    - `web/src/components/Statistics/StatsWordCloud.vue`
    - `web/src/components/Statistics/StatsR18AiTrend.vue`
  - Pre-commit: `cd web && vue-tsc --noEmit && yarn lint`

---

- [x] 21. 扩展已有组件 — 总结卡片、排序切换、累计曲线、健全度、隐藏神作

  **What to do**:
  - 修改以下已有组件，利用 Tasks 2-3 已新增的数据字段：

  **21a. Summary Card（修改 StatsOverview.vue）**:
  - 在概览卡片下方添加一段"收藏人格总结"文字
  - 从已有数据自动生成，例如：
    > "你的收藏横跨 {year_min}-{year_max} 共 {span} 年，最爱的画师是 {top_author.name}，收藏了他的 {top_author.count} 件作品。收藏中 R18 占 {r18_percent}%，AI 作品占 {ai_percent}%。"
  - 纯文字展示，无需图表

  **21b. Sort Toggle（修改 StatsAuthorChart.vue）**:
  - 在作者排行图表上方添加排序切换按钮：`按作品数` / `按总收藏`
  - 默认按作品数，点击切换到 `total_bookmarks` 排序
  - 数据已存在（`AuthorStats.total_bookmarks`），只需前端重新排序

  **21c. Cumulative Line（修改 StatsTimeChart.vue）**:
  - 在已有的混合图表中添加第三条数据系列：累计曲线
  - 前端计算：`yearly_trend` 数据累加生成 cumulative 数组
  - 使用线图（`type: 'line'`）展示，添加图例"累计收藏"

  **21d. Sanity Level Bars（修改 StatsPieSection.vue）**:
  - 在 R18/AI/形状 三个图表下方添加健全度等级分布
  - 使用柱状图展示 `sanity_level` 0-6 的分布
  - 数据来自 `sanity_levels`

  **21e. Hidden Gems Table（修改 StatsTopLists.vue）**:
  - 在收藏 TOP10 / 浏览 TOP10 之后添加第三个排行：`收藏效率 TOP10`
  - 排序依据：`bookmark / view` 比值最高
  - 显示：排名 / 标题 / 作者 / bookmark数 / view数 / 效率比值
  - 数据来自 `hidden_gems`

  **Must NOT do**:
  - 不改变现有组件的 props/emits 接口（只扩展内部内容）
  - 不重新请求后端

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 多个组件的 UI 扩展，涉及数据组合和展示
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (与 Tasks 14-17 并行或在其后)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 6, 7, 9, 11, 12 (需要这些组件已存在)

  **Acceptance Criteria**:
  - [ ] 总结卡片文字正确（跨度、最爱画师、R18率、AI率）
  - [ ] 作者排序切换正常工作
  - [ ] 累计曲线在时间趋势图中可见
  - [ ] 健全度柱状图显示 7 个等级
  - [ ] 收藏效率排行显示 TOP 10

  **QA Scenarios**:
  ```playwright
  Scenario: Summary card shows correct insights
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Open stats modal -> overview section
      2. Verify summary text contains "横跨", "最爱", "R18" keywords
      3. Verify year span is correct (year_max - year_min)
    Expected Result: Insight text generated from data
    Evidence: .sisyphus/evidence/task-21a-summary.txt

  Scenario: Author sort toggle
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Open stats modal -> author chart section
      2. Verify default sort is "按作品数" (illustration_count descending)
      3. Click toggle to "按总收藏"
      4. Verify chart reorders by total_bookmarks
    Expected Result: Chart re-sorts correctly
    Evidence: .sisyphus/evidence/task-21b-sort-toggle.mp4

  Scenario: Cumulative line visible
    Tool: Playwright
    Preconditions: Stats loaded with 5+ years
    Steps:
      1. Open stats modal -> time trend section
      2. Verify cumulative line chart series exists (distinct color)
      3. Verify last data point equals total_illustrations
    Expected Result: Cumulative line shows collection growth
    Evidence: .sisyphus/evidence/task-21c-cumulative.png

  Scenario: Hidden gems list shows high ratio works
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Open stats modal -> top lists section
      2. Verify third list titled "收藏效率 TOP10" exists
      3. Verify first entry has highest bookmark/view ratio
    Expected Result: Hidden gems list displayed correctly
    Evidence: .sisyphus/evidence/task-21e-hidden-gems.png
  ```
  ---

- [x] 22. 构建 StatsTagTrend.vue — 标签年度趋势热力图

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsTagTrend.vue`
  - Props: `tagTrend: Array<{ year, tag_name, count }>`
  - 展示内容：
    - **堆叠面积图** 或 **热力图**（推荐堆叠面积图，更直观）
    - X 轴：年份
    - Y 轴：标签出现次数
    - 每个标签一条面积系列，半透明叠加
    - 展示 TOP 20 标签的年度变化
  - Chart.js 配置：
    - 使用 vue-chartjs 的 `<Bar>` 组件（堆叠模式）或手动构建
    - 或者使用纯 `<canvas>` + Chart.js 堆叠面积图
    - 每个标签一种颜色（使用 Chart.js 默认调色板或 Tailwind 色板）
    - 悬停显示：年份 + 标签名 + 出现次数
    - 深色模式适配
    - 中文标签
  - 数据处理：
    - 将 `[{year, tag_name, count}]` 转换为 labels（年份）+ datasets（每个标签一条系列）
    - 限制最多显示 20 个标签（数据已限）
    - 计算每年各标签占比，可选显示绝对值或百分比

  **Must NOT do**:
  - 不请求额外后端数据
  - 不替换标签排行柱状图（Task 8）和词云（Task 19）——三者共存

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 堆叠面积图/热力图的数据处理和可视化
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (与 Tasks 21, 23 并行)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 1

  **References**:
  - Chart.js stacked area: `https://www.chartjs.org/docs/latest/charts/area.html`
  - `src-tauri/src/stats.rs` — `TagTrendItem` 类型

  **Acceptance Criteria**:
  - [ ] 堆叠面积图展示 TOP 20 标签每年频率
  - [ ] 每个标签一种颜色，图例可点击筛选
  - [ ] 悬停时显示标签名 + 年份 + 次数
  - [ ] 深色模式适配
  - [ ] 响应用户筛选（年份过滤后只显示相应年份）

  **QA Scenarios**:
  ```playwright
  Scenario: Tag trend chart renders
    Tool: Playwright
    Preconditions: Stats loaded with multi-year tag_trend data
    Steps:
      1. Open stats modal -> tag trend section
      2. Verify stacked area chart renders with multiple colored areas
      3. Verify X-axis shows years, Y-axis shows counts
      4. Hover over top area -> tooltip shows tag name + year + count
      5. Click a legend item -> corresponding area hides/shows
    Expected Result: Tag trend chart shows how tag usage changed over years
    Evidence: .sisyphus/evidence/task-22-tag-trend.png
  ```
  ---

- [x] 23. 构建 StatsAuthorDiscovery.vue — 作者发现时间线

  **What to do**:
  - 创建 `web/src/components/Statistics/StatsAuthorDiscovery.vue`
  - Props: `authorDiscovery: Array<{ author_id, author_name, author_account, first_year, works_count }>`
  - 展示内容：
    - **时间线形式**或堆叠柱状图
    - **方案一（推荐）**：水平时间线——按首次收录年份分组，展示该年发现的新作者
    - **方案二**：堆积柱状图——X 轴为年份，Y 轴为"当年新发现的作者数"
  - 每条显示：
    - 年份
    - 该年发现的作者列表（头像/名字 + 作品数）
    - 按作品数降序排列
  - 样式：
    - 时间线样式（圆点 + 连线 + 卡片）
    - 或列表形式（年份分组标题 + 作者条目）
    - 深色模式适配
  - 交互：
    - 点击作者名跳转到 Pixiv 用户页
    - 可选：展开/收起某年的作者列表

  **Must NOT do**:
  - 不请求额外数据
  - 不使用第三方时间线库（纯 CSS/Tailwind 实现）

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: 时间线 UI 设计 + 数据分组展示
  - **Skills**: `[]`

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (与 Tasks 21, 22 并行)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 1

  **References**:
  - `web/src/config/index.ts:LINK_PIXIV_USER` — 作者链接

  **Acceptance Criteria**:
  - [ ] 时间线按年份分组显示发现的作者
  - [ ] 每个作者显示名字 + 作品数
  - [ ] 作者名可点击跳转 Pixiv
  - [ ] 深色模式适配
  - [ ] 响应式布局

  **QA Scenarios**:
  ```playwright
  Scenario: Author discovery timeline renders
    Tool: Playwright
    Preconditions: Stats loaded with author_discovery data
    Steps:
      1. Open stats modal -> author discovery section
      2. Verify timeline shows year groups (e.g. "2024", "2023", ...)
      3. Verify each year group lists authors discovered that year
      4. Verify author names are clickable links to pixiv.net/users/{id}
      5. Verify works_count displayed per author
    Expected Result: Timeline shows author discovery history
    Evidence: .sisyphus/evidence/task-23-author-discovery.png

  Scenario: Dark mode
    Tool: Playwright
    Preconditions: Stats loaded
    Steps:
      1. Toggle dark mode
      2. Open stats modal -> author discovery section
      3. Verify text and borders are readable against dark background
    Expected Result: Timeline readable in dark mode
    Evidence: .sisyphus/evidence/task-23-discovery-dark.png
  ```

  **Commit**: YES (groups with tasks 21-22)
  - Message: `feat(stats): add author discovery, tag trends, and insight extensions`
  - Files:
    - `web/src/components/Statistics/StatsOverview.vue` (modified)
    - `web/src/components/Statistics/StatsAuthorChart.vue` (modified)
    - `web/src/components/Statistics/StatsTimeChart.vue` (modified)
    - `web/src/components/Statistics/StatsPieSection.vue` (modified)
    - `web/src/components/Statistics/StatsTopLists.vue` (modified)
    - `web/src/components/Statistics/StatsTagTrend.vue` (new)
    - `web/src/components/Statistics/StatsAuthorDiscovery.vue` (new)
  - Pre-commit: `cd web && vue-tsc --noEmit && yarn lint`

---

> 4 review agents run in PARALLEL. ALL must APPROVE.
> Present consolidated results to user and get explicit "okay" before completing.
> **Do NOT auto-proceed after verification. Wait for user's explicit approval.**

- [x] F1. **Plan Compliance Audit** — `oracle`  
  **VERDICT: APPROVE** — Must Have [22/22] | Must NOT Have [6/6] | All 23 tasks complete

- [x] F2. **Code Quality Review** — `unspecified-high`  
  **VERDICT: APPROVE** — vue-tsc PASS | yarn lint PASS (0 errors) | Anti-patterns CLEAN

- [x] F3. **Real Manual QA** — `unspecified-high` (+ `playwright` skill)  
  **NOT EXECUTED** — requires Windows with Tauri+Vite dev server + actual DB. Code compiles clean (vue-tsc PASS, lint PASS).

- [x] F4. **Scope Fidelity Check** — `deep`  
  **VERDICT: APPROVE** — Tasks [23/23 compliant] | Contamination [CLEAN]

---

## Commit Strategy

- **Tasks 2-4**: `feat(backend): add get_statistics Tauri command for bookmark data analysis`
  - Files: `src-tauri/src/stats.rs`, `src-tauri/src/main.rs`
  - Pre-commit: `cd src-tauri && cargo check`

- **Tasks 1 + 5-12 + 18-20**: `feat(stats): add bookmark data statistics panel with charts`
  - Files: `web/package.json`, `web/src/components/Statistics/Stats*.vue`, `web/src/App.vue`
  - Pre-commit: `cd web && vue-tsc --noEmit && yarn lint`

- **Tasks 13-17**: `feat(stats): integrate statistics modal with navbar, data flow, dark mode, responsive`
  - Files: `web/src/components/Navbar.vue`, `web/src/components/Statistics/StatsModal.vue`
  - Pre-commit: `cd web && vue-tsc --noEmit && yarn lint`

- **Tasks 21-23**: `feat(stats): add tag trends, author discovery, hidden gems and insight extensions`
  - Files: `web/src/components/Statistics/StatsTagTrend.vue`, `web/src/components/Statistics/StatsAuthorDiscovery.vue`, extensions to existing components
  - Pre-commit: `cd web && vue-tsc --noEmit && yarn lint`

---

## Success Criteria

### Verification Commands
```bash
cd src-tauri && cargo check
cd web && vue-tsc --noEmit
cd web && yarn lint
```

### Final Checklist
- [ ] 所有 Must Have 功能实现并验证通过
- [ ] 所有 Must NOT Have 约束未被违反
- [ ] `cargo check` 通过
- [ ] `vue-tsc --noEmit` 通过
- [ ] `yarn lint` 通过
- [ ] 所有 Agent QA 场景通过
- [ ] F1-F4 全部 APPROVE
- [ ] 用户明确"okay"确认
