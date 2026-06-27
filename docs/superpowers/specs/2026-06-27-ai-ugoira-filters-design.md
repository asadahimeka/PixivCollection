# AI & Ugoira Sidebar Filters

## Overview

Add two new toggle filters to the sidebar alongside the existing R18 filter:

- **AI 作品** — three-state select (隐藏/显示/仅显示), filtering on `i.is_ai`
- **动图作品** — simple toggle (only mode), filtering on `i.ext = 'zip'`

## Changes

### Rust backend (`src-tauri/src/query.rs`)

1. Add `is_ai: Option<String>` to `ImageQuery` struct
2. Add `ext: Option<String>` to `ImageQuery` struct
3. In `build_where()`:
   - `is_ai = "hidden"` → `AND i.is_ai = 0`
   - `is_ai = "only"` → `AND i.is_ai = 1`
   - `ext = "ugoira"` → `AND i.ext = 'zip'`

### Frontend store (`web/src/store/index.ts`)

1. Add `restrict.isAi: 'hidden' | 'show' | 'only'` (default `'show'`) to `filterConfig`
2. Add `restrict.ugoira: boolean` (default `false`) to `filterConfig`
3. In `buildFilterQuery()`: pass `is_ai` and `ext` to query

### Frontend sidebar UI (`web/src/components/Sidebar/Sidebar.vue`)

1. AI select — same pattern as R18, three options (隐藏/显示/仅显示)
2. Ugoira toggle — single select (仅显示动图 / 不过滤)

### Settings persistence (`web/src/App.vue`)

1. The new fields live under `restrict` key which is already persisted via `restrictConfig` setting

## Fields already exist

| Item | DB column | ImageRow | Frontend type |
|------|-----------|----------|---------------|
| `is_ai` | ✅ `is_ai INTEGER` + index | ✅ `pub is_ai: bool` | ✅ `isAI?: boolean` |
| `ext` | ✅ `ext TEXT` | ✅ `pub ext: String` | ✅ `ext: string` |

## Data flow (same as R18)

```
Sidebar control → filterConfig → buildFilterQuery() → invoke('query_images', query)
  → ImageQuery → build_where() → SQL WHERE clause → results
```
