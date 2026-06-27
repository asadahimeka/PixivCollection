# AI & Ugoira Sidebar Filters — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add AI作品 (is_ai) and 动图作品 (ext='zip') sidebar filters matching existing R18 three-state pattern.

**Architecture:** Frontend filter state flows through Pinia store → `buildFilterQuery()` → Tauri `invoke('query_images', query)` → Rust `ImageQuery` → `build_where()` adds SQL WHERE clause. Both AI and ugoira follow the exact same path as the existing R18 filter.

**Tech Stack:** Rust (rusqlite), TypeScript (Vue 3 + Pinia)

**Global Constraints:**
- Match existing R18 filter pattern exactly
- `is_ai` and `ext` columns already exist in DB schema
- `is_ai` already indexed, returned in ImageRow, and mapped in frontend `Image` type
- `ext` already returned in ImageRow and frontend `Image` type
- Filter must support SQL-level filtering (no client-side post-filter)

---

### Task 1: Rust backend — Add `is_ai` and `ext` to ImageQuery + WHERE clauses

**Files:**
- Modify: `src-tauri/src/query.rs:17-47` (struct) + after line 196 (WHERE clauses)

**Interfaces:**
- Consumes: existing `ImageQuery` struct, `build_where()`, `push_param()` patterns
- Produces: `ImageQuery.is_ai: Option<String>`, `ImageQuery.ext: Option<String>`, new WHERE branches

- [ ] **Step 1: Add `is_ai` and `ext` fields to ImageQuery struct**

After line 41 (`pub max_sanity_level: Option<i64>,`), add:
```rust
    /// `"hidden"` (hide AI), `"only"` (AI only), or `"show"` (no filter).
    pub is_ai: Option<String>,
    /// `"ugoira"` (only ugoira/animated), or absent (no filter).
    pub ext: Option<String>,
```

- [ ] **Step 2: Add `is_ai` WHERE clause in `build_where()`**

After the R18 block (after line 196's `}`), add:
```rust
        // ---- AI ----
        if let Some(ref val) = self.is_ai {
            match val.as_str() {
                "hidden" => sql.push_str(" AND i.is_ai = 0"),
                "only" => sql.push_str(" AND i.is_ai = 1"),
                _ => { /* "show" or unknown – no filter */ }
            }
        }
```

- [ ] **Step 3: Add `ext` WHERE clause after the AI block**

```rust
        // ---- Ugoira ----
        if let Some(ref val) = self.ext {
            if val == "ugoira" {
                sql.push_str(" AND i.ext = 'zip'");
            }
        }
```

Pattern matches existing R18 code: hardcoded SQL (no parameter binding needed since values are enum-like strings controlled by our code, not user input).

---

### Task 2: Frontend — Add filter state + query builder + UI + persistence

**Files:**
- Modify: `web/src/store/index.ts:99-102` (filterConfig.restrict), line 296-297 (buildFilterQuery)
- Modify: `web/src/components/Sidebar/Sidebar.vue:197-223` (UI controls after R18)

**Interfaces:**
- Consumes: existing `filterConfig` structure, `buildFilterQuery()` pattern
- Produces: filter state + query + UI controls

- [ ] **Step 1: Add state to filterConfig.restrict**

In `web/src/store/index.ts:99-102`, change to:
```ts
restrict: {
  maxSanityLevel: 6,
  r18: 'show' as 'hidden' | 'show' | 'only',
  isAi: 'show' as 'hidden' | 'show' | 'only',
  ugoira: false as boolean,
},
```

- [ ] **Step 2: Add fields to buildFilterQuery()**

In `web/src/store/index.ts`, after line 297 (`q.max_sanity_level = ...`), add:
```ts
q.is_ai = this.filterConfig.restrict.isAi
if (this.filterConfig.restrict.ugoira) {
  q.ext = 'ugoira'
}
```

- [ ] **Step 3: Add UI controls in Sidebar.vue**

After the R18 `</select>` (line 222), add inside the same `<SidebarBlock>`:
```html
        AI:
        <select
          v-model="filterConfig.restrict.isAi"
          class="mx-1 rounded-md border px-1 py-0.5 transition-colors hover:border-blue-500 dark:border-white/20 dark:bg-[#1a1a1a] dark:hover:border-blue-500"
        >
          <option value="hidden">
            隐藏
          </option>
          <option value="show">
            显示
          </option>
          <option value="only">
            仅显示
          </option>
        </select>
        动图:
        <label class="inline-flex cursor-pointer items-center">
          <input
            v-model="filterConfig.restrict.ugoira"
            type="checkbox"
            class="mx-1 rounded border-gray-300 text-blue-600 focus:ring-blue-500 dark:border-white/20 dark:bg-[#1a1a1a]"
          >
          <span class="text-sm">仅显示动图</span>
        </label>
```

- [ ] **Step 4: Verify persistence**

`App.vue:524-526` already registers `restrictConfig` with `deepMerge: true`, so new fields under `restrict.*` are persisted automatically. No App.vue change needed.
