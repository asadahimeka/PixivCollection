# PixivCollection 十万级收藏夹性能优化方案

> 当前架构在收藏夹数量达到 10 万+ 条时，存在初始化加载慢、筛选卡顿的问题。
> 本文档分析根因并提出多条优化路径，供评估和实施参考。

---

## 一、根因分析

### 1.1 数据加载链路

```
images.json (20-40MB) → readTextFile → JSON.parse → window.__fullImages__[100k items]
```

- 整个 JSON 文件在 **前端主线程** 同步解析，10 万条数据约 20-40MB
- `JSON.parse` 阻塞主线程数秒，期间 UI 完全无响应
- 全部 10 万条对象常驻内存，占用 ~200-400MB JS heap

### 1.2 筛选链路

```
用户切换筛选条件 → loadFilteredImages() → for (100k items) { imageFilter() } → 更新渲染
```

- `imageFilter()` 每次执行 6 项检查（R18、搜索、年份、作者、标签、形状、尺寸）
- `getSearchStr()` 对每张图拼接 `id + title + author.id + author.name + tags + tag translations` 字符串，然后 `.includes()` 子串搜索
- 10 万 × 多次字符串操作 = **每次筛选阻塞主线程 300ms-3s**
- `updateFullCounts()` 又额外遍历全量做 `new Set()` 去重统计

### 1.3 问题本质

架构假设了「一次加载，前端全量处理」的模式。这在 1 万条以下可行，在 10 万条级别每个操作都是 O(n) 全表扫描，主线程被大量计算阻塞。

---

## 二、方案总览

| 方案 | 改造成本 | 收益 | 推荐顺序 |
|------|---------|------|---------|
| 预计算搜索字符串 + 索引 | **低** | 中 | 第 1 步 |
| Web Worker 异步筛选 | **中** | 高 | 第 2 步 |
| Rust SQLite 存储 + 查询 | **高** | **极高** | 第 3 步 |
| 分块 JSON + 惰性加载 | 中 | 中 | 备选 |
| Rust warp 端切片查询 | 高 | 中 | 备选 |

---

## 三、方案详述

### 方案一：预计算搜索字符串 + 哈希索引（低投入，快速见效）

#### 改动量：约 50-100 行，不改架构

#### 做法

在 `init()` 加载完数据后，**一次性** 完成以下预处理：

```typescript
// 在 init() 末尾执行，只运行一次
for (const img of window.__fullImages__) {
  // 预计算搜索字符串
  img.searchStr = computeSearchStr(img)  // 只拼接一次
}

// 按作者 ID 建索引
const authorIndex = new Map<number, number[]>()
for (let i = 0; i < window.__fullImages__.length; i++) {
  const img = window.__fullImages__[i]
  if (!authorIndex.has(img.author.id)) {
    authorIndex.set(img.author.id, [])
  }
  authorIndex.get(img.author.id)!.push(i)
}
```

筛选时直接使用索引：

```typescript
// 原来是 O(n) 全表扫描
window.__fullImages__.filter(img => img.author.id === targetId)

// 优化后 O(1) 哈希查找
const indices = authorIndex.get(targetId) || []
const results = indices.map(i => window.__fullImages__[i])
```

#### 收益
- 作者筛选：从 O(n) → O(1)
- 搜索（id/标题/标签）：`includes()` 不再重复拼接字符串，但仍然是 O(n) 子串扫描
- 年份/形状/尺寸筛选：仍然需要 O(n) 遍历

#### 局限性
- 搜索仍然是全表扫描，没有从根本上解决
- 多条件组合筛选（如「2022 年 R18 且收藏 > 5000」）仍然要依次过滤
- 内存占用增加（每个 image 多一个 `searchStr` 字段）

---

### 方案二：Web Worker 异步筛选（中等投入，大幅提升 UX）

#### 改动量：新增一个 worker 文件 + 修改 store，约 150 行

#### 做法

将 `loadFilteredImages()`、`imageFilter`、`getSearchStr`、`updateCounts` 全部移入 Web Worker。

```typescript
// filter.worker.ts
self.onmessage = (e) => {
  const { images, filterConfig, action } = e.data

  if (action === 'filter') {
    const results = []
    for (const img of images) {
      if (imageFilter(img, filterConfig)) {
        results.push(img)
      }
    }
    // 只传回 id+part，不传整个对象
    self.postMessage({
      type: 'filtered',
      ids: results.map(img => `${img.id}_${img.part}`),
      counts: computeCounts(results)
    })
  }
}
```

主线程改为消息驱动：

```typescript
// store/index.ts
const filterWorker = new Worker(
  new URL('./filter.worker.ts', import.meta.url), { type: 'module' }
)

filterWorker.onmessage = (e) => {
  if (e.data.type === 'filtered') {
    const ids = new Set(e.data.ids)
    // 从全量数据中只取需要的那部分
    this.imagesFiltered = window.__fullImages__.filter(
      img => ids.has(`${img.id}_${img.part}`)
    )
    this.filteredCounts = e.data.counts
  }
}
```

#### 收益
- 筛选计算**完全不阻塞主线程** → UI 始终可交互
- Worker 有自己的 V8 堆，不会与渲染线程争抢内存
- `getSearchStr` 等 CPU 密集型操作不影响页面滚动/动画

#### 注意事项
- Worker 不能访问 DOM、`window.__fullImages__`，需要通过 `postMessage` 传入数据
- 传递 10 万条数据到 Worker 有序列化开销（~200ms），但这是一次性成本，之后增量更新即可
- `unplugin-auto-import` 在 Worker 中可能不生效，需要手动 import

---

### 方案三：Rust SQLite 存储 + 查询（高投入，终局方案） ⭐ 推荐

#### 改动量：约 400-600 行 Rust 代码 + 前端修改

#### 架构

```
images.json (源文件)
    │
    ▼
[首次启动] Rust ingest → SQLite 数据库 (data/images.db)
    │
    ▼
Tauri invoke({ filters, page, pageSize }) → Rust 查询 → 返回 30 条数据
```

#### SQLite 表结构

```sql
CREATE TABLE images (
    id INTEGER NOT NULL,
    part INTEGER NOT NULL,
    title TEXT NOT NULL,
    author_id INTEGER NOT NULL,
    author_name TEXT NOT NULL,
    author_account TEXT NOT NULL,
    bookmark INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    ext TEXT NOT NULL,
    sanity_level INTEGER NOT NULL,
    x_restrict INTEGER NOT NULL,
    img_s TEXT,
    img_m TEXT,
    img_l TEXT,
    img_o TEXT,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    is_ai INTEGER DEFAULT 0,
    PRIMARY KEY (id, part)
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    translated_name TEXT
);

CREATE TABLE image_tags (
    image_id INTEGER NOT NULL,
    image_part INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    FOREIGN KEY (image_id, image_part) REFERENCES images(id, part),
    FOREIGN KEY (tag_id) REFERENCES tags(id)
);

-- 索引
CREATE INDEX idx_images_author_id ON images(author_id);
CREATE INDEX idx_images_created_at ON images(created_at);
CREATE INDEX idx_images_bookmark ON images(bookmark);
CREATE INDEX idx_images_x_restrict ON images(x_restrict);
CREATE INDEX idx_images_sanity_level ON images(sanity_level);
CREATE INDEX idx_images_size ON images(width, height);
CREATE INDEX idx_images_is_ai ON images(is_ai);
CREATE INDEX idx_tag_name ON tags(name);
CREATE INDEX idx_image_tags ON image_tags(image_id, image_part, tag_id);

-- 全文搜索（可选）
CREATE VIRTUAL TABLE images_fts USING fts5(
    title, author_name, author_account
);
```

#### Rust 查询接口

```rust
#[derive(Deserialize)]
pub struct FilterParams {
    pub search: Option<String>,
    pub author_id: Option<i32>,
    pub tag: Option<String>,
    pub year: Option<i32>,
    pub shape: Option<String>,         // horizontal / vertical / square / ratio-4:3 ...
    pub width_min: Option<i32>,
    pub width_max: Option<i32>,
    pub height_min: Option<i32>,
    pub height_max: Option<i32>,
    pub bookmark_min: Option<i32>,
    pub r18: String,                   // 'hidden' | 'show' | 'only'
    pub max_sanity_level: Option<i32>,
    pub sort_by: String,               // 'id_desc' | 'id_asc' | 'bookmark_desc'
}

#[derive(Deserialize)]
pub struct PageParams {
    pub page: u32,
    pub page_size: u32,                // 默认 30
}

#[tauri::command]
fn query_images(
    filters: FilterParams,
    page: PageParams,
) -> Result<QueryResult, String> {
    // 构建动态 SQL
    let mut sql = String::from("
        SELECT i.* FROM images i
        LEFT JOIN image_tags it ON i.id = it.image_id AND i.part = it.image_part
        LEFT JOIN tags t ON it.tag_id = t.id
        WHERE 1=1
    ");
    // 动态追加条件...（使用 rusqlite 的 params 绑定）
    sql += " LIMIT ? OFFSET ?";
    // 返回 QueryResult { images: Vec<Image>, total: u64 }
}

#[tauri::command]
fn get_filter_stats(filters: FilterParams) -> Result<Counts, String> {
    // SELECT COUNT(DISTINCT id), COUNT(DISTINCT author_id), COUNT(DISTINCT ...)
    // 用 SQL 聚合替代前端遍历
}
```

#### 前端修改

```typescript
// 替换 store 中的 loadFilteredImages
async function applyFilter() {
  const result = await invoke('query_images', {
    filters: getFilterParams(),
    page: { page: 1, page_size: 30 },
  })
  store.imagesFiltered = result.images
  store.totalCount = result.total
  // 不再需要 window.__fullImages__ / __filteredImages__
}
```

#### 收益

| 操作 | 当前（前端全量） | 优化后（SQLite） |
|------|----------------|-----------------|
| 首次加载 | 20-40MB JSON parse，~2-5s | 立即显示，后台 ingest < 10s |
| 筛选（单条件） | O(n) 全表遍历，300ms-3s | **毫秒级**（索引 B-tree 查找） |
| 搜索 | `includes()` 子串扫描，O(n) | **毫秒级**（FTS5 全文索引） |
| 组合筛选 | 更慢，n 个条件串行 | **仍然毫秒级**（SQL 优化器多索引） |
| 分页加载 | 当前已部分实现（30 条一批） | 不变，但来自 Rust 而非前端 slice |
| 统计计数 | `new Set()` 遍历全量 | **即时**（`SELECT COUNT(DISTINCT)`） |
| 内存占用 | ~200-400MB（所有图片对象） | **~3-5MB**（仅当前页 30 条） |

#### 实现步骤

1. **添加依赖**：`Cargo.toml` 中加入 `rusqlite = { version = "0.31", features = ["bundled"] }`
   - `bundled` 特性会静态编译 SQLite，不需要系统预装
   - 增加编译时间 ~2min，二进制体积增加 ~2MB

2. **Ingest 函数**：解析 `images.json`，逐条写入 SQLite
   ```
   fn ingest_images(json_path: &str, db_path: &str) -> Result<(), String>
   ```
   - 使用事务批量插入（每 1000 条 commit 一次）
   - 10 万条数据插入约 3-5s
   - 记录一个版本号，`images.json` 未变则跳过 ingest

3. **查询命令**：实现 `query_images` 和 `get_filter_stats`，动态构建带参数绑定的 SQL

4. **前端改造**：
   - 移除以 `window.__fullImages__` 和 `__filteredImages__` 为核心的架构
   - 所有筛选操作改为 `invoke('query_images', ...)` + 响应式更新
   - 用户点击"加载更多" → `page++` → 请求下一页

5. **增量更新**：每次 pxder 下载完成后，比对增量数据更新 SQLite，而不是全量重入

---

### 方案四：分块 JSON + 惰性加载（备选，独立于 SQLite）

#### 做法

将 `images.json` 按年/月拆分为多个文件：

```
data/
  images.idx.json   # 索引文件（很小）
  images_2024.json
  images_2023.json
  images_2022.json
  ...
```

索引文件结构：
```json
{
  "version": 2,
  "total": 123456,
  "chunks": [
    { "year": 2024, "count": 12300, "file": "images_2024.json" },
    { "year": 2023, "count": 23400, "file": "images_2023.json" },
  ]
}
```

初始化时只加载索引文件（~1KB），UI 立即展示。后续根据用户筛选条件按需加载对应分块。

#### 收益
- 首次加载几乎瞬间完成
- 浏览某年份数据只需加载该年份分块
- 可与方案一（预计算 + 索引）组合使用

#### 局限性
- 跨分块查询复杂（如「2020-2024 年所有 R18」需合并多个分块）
- 不是真正的随机访问，筛选能力受限
- 需要修改 pxder 的 pipeline（rename.mjs 输出分块）

---

## 四、推荐实施路径

### Phase 1（快速改善，1-2 天）

```
预计算 searchStr + 作者哈希索引 + 标签哈希索引
```

**目标**：用最小的代码改动消除最明显的卡顿。

### Phase 2（本质改善，3-5 天）

```
SQLite 存储 + Rust 查询 + 前端分页
```

**目标**：从根本上将 O(n) 前端全量扫描改为 O(log n) 数据库索引查询。

### Phase 3（可选增强，1-2 天）

```
Web Worker 用于过渡期
```

**目标**：如果在 Phase 2 之前需要快速缓解问题，先用 Worker 解决 UI 阻塞。

---

## 五、预判 & 风险

| 风险 | 说明 | 缓解措施 |
|------|------|---------|
| SQLite bundled 编译慢 | 首次 `cargo build` 需编译 SQLite C 代码 | 仅影响首次构建，后续增量编译很快 |
| 用户已有大量数据 | 现有 `images.json` 需要迁移到 SQLite | 提供一次性迁移命令，显示进度条 |
| 增量同步 | pxder 下载后需同步到 SQLite | `images.json` 版本号检测；或在 rename.mjs 中触发 Tauri 事件通知 Rust 端更新 |
| 文件路径平台差异 | Windows 路径反斜杠 | Rust `std::path::Path` 统一处理 |
