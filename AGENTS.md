# AGENTS.md — PixivCollection

## Project overview

A Tauri v1 desktop app that scrapes Pixiv bookmarks to a local SQLite database, converts images to WebP and ugoira to MP4, and provides a paginated gallery browser. Images are queried from SQLite with server-side filtering — no in-memory full list.

Three packages in one monorepo:

| Directory | Role | Stack |
|-----------|------|-------|
| `web/` | Desktop UI (Vue 3 SPA) | Vue 3 + TS + Vite + Pinia + Tailwind CSS |
| `src-tauri/` | Tauri Rust backend | Tauri v1 + rusqlite + warp (local HTTP server) |
| `pxder/` | Pixiv scraper (vendored submodule) | Node.js + pixiv-api-client + sharp + ffmpeg |

## Commands

```bash
# Dev (launches Tauri with Vite dev server on :8080)
yarn dev

# Production build for Windows
yarn build:win

# Typecheck + build web frontend only (inside web/)
cd web && yarn build    # vue-tsc && vite build

# Dev server only (web/, standalone)
cd web && yarn dev

# Lint web/ only
cd web && yarn lint     # eslint src --fix
```

**Important**: `vue-tsc` is part of the build — type errors block production builds. Run `vue-tsc --noEmit` in `web/` before committing.

## Architecture notes

### Data flow

1. User sets RefreshToken + storage path → saved to `pxder/src/config/config.json`
2. "Update bookmark" spawns `cmd /C start.bat` in pxder/ directory
3. start.bat: `pxder --debug -b` (scrape) → `rename.mjs` (build `data/images.json`) → `compress.mjs` (sharp: images→webp, ffmpeg: zip→mp4)
4. On app startup, `ensure_db` Tauri command reads `images.json` and populates a local SQLite database (`images.db` in the configured imgDir). Emits `import-progress` events to the frontend during initial import or reimport.
5. Gallery queries images via `query_images` with SQL-level filtering, sorting, and pagination (offset/limit 60). Multi-page continuation fetched automatically in the same command.
6. Tags are batch-fetched in a second query using the returned image IDs.

### Image data

- Images stored in SQLite table `images` with columns: `id`, `part`, `len`, `title`, `width`, `height`, `ext`, `author_id`, `author_name`, `author_account`, `bookmark`, `view`, `created_at`, `sanity_level`, `x_restrict`, `is_ai`, image URLs, `img_order`
- Multi-page illustrations flattened to one row per page (`part`/`len` fields)
- Tags in separate `tags` table; many-to-many via `image_tags`
- Locally cached images named: `({id}){title}_p{part}.webp` (or `.mp4` for ugoira)
- Path: `{imgDir}/bookmark_webp/`, `{imgDir}/bookmark_ugoira/`

### Config storage

- `@orilight/vue-settings` (prefix `PXCT`) persists to localStorage automatically
- Settings registered in `App.vue` `onMounted` — always register new settings there
- Window `__CONFIG__` injected at HTML load from localStorage keys (`__PXCT_*`): contains `imgDir`, `pxderToken`, `pxderProxy`

### State management (Pinia store)

- Single store at `web/src/store/index.ts`
- Store holds: `imagesFiltered` (paginated view), filter config, masonry config, color scheme, viewer state, image counts
- Filtering is server-side via `buildFilterQuery()` → `query_images` Tauri command — no in-memory filtering
- Types in `web/src/types/index.d.ts` (global `Image`, `Tag`, etc.)
- Config constants in `web/src/config/index.ts`

### Local HTTP server

- Tauri command `start_local_server` runs warp on `127.0.0.1:32154`
- Serves static files (cached WebP/MP4) from the configured image directory
- Only starts once per session (guarded by `sessionStorage`)
- Controlled by `masonryConfig.loadImageByLocalHttp`

### Tauri specifics (v1)

- `withGlobalTauri: true` → access via `window.__TAURI__` or top-level `__TAURI__`
- Allowlist: all APIs enabled (fs, path, shell, http, protocol)
- Window: 1280x800 default, min 375x600, centered
- Plugins: `window-state` (persists window position/size), `single-instance`
- Rust modules: `db.rs` (schema, import, connection pool), `query.rs` (filter builder, ImageRow), `stats.rs` (statistics computation), `main.rs` (commands)

**All Tauri commands:**

| Command | Args | Returns | Description |
|---------|------|---------|-------------|
| `ensure_db` | imgDir, version | EnsureDbResult | Import images.json into SQLite if needed |
| `query_images` | query (ImageQuery) | QueryResult | Paginated gallery query with filters, sort, tags |
| `query_image_counts` | query (ImageQuery) | CountsResult | Aggregate counts for current filter |
| `get_full_counts` | none | JSON | Cached unfiltered counts from _meta |
| `get_filter_options` | none | FilterOptions | Cached sidebar filter options (years, authors, tags) |
| `search_tags` | query | Vec<TagSuggestion> | Tag autocomplete search |
| `refresh_caches` | imgDir, version | () | Recompute _meta cache after import |
| `reimport_db` | imgDir, version | ImportResult | Full reimport from images.json |
| `import_json_to_db` | imgDir, jsonContent | ImportResult | Import a specific JSON blob |
| `get_statistics` | yearMin, yearMax, r18, isAi | StatisticsResult | Bookmark statistics (18 query types) |
| `start_local_server` | base | () | Start warp HTTP server for local images |
| `get_executable_dir` | none | PathBuf | App executable directory |
| `restart_app` | none | () | Restart the application |

### Rust backend structure

| File | Purpose |
|------|---------|
| `db.rs` | SQLite schema creation, indexing, JSON import, connection management (`get_conn()` singleton) |
| `query.rs` | `ImageQuery` filter struct, SQL builder, `ImageRow`/`QueryResult`/`CountsResult` types, `build_sql()` generates parameterised queries |
| `stats.rs` | `StatsFilter` + `compute_statistics()` — 18 SQL queries (author ranking, tag ranking, yearly trend, distribution, R18/AI ratio, shape, top works, hidden gems, tag trend, author discovery, sanity levels) |
| `main.rs` | All Tauri commands, warp server startup, `generate_handler!` registration |

### Frontend conventions

- **ESLint**: `@antfu/eslint-config-vue` — single quotes, no semicolons, brace-style `1tbs`
- **Formatting**: ESLint fix on save via VSCode, no Prettier
- **Auto-imports**: Vue APIs + vue-router auto-imported (see `auto-imports.d.ts`)
- **Component auto-registration**: `unplugin-vue-components` — use PascalCase in templates directly
- **Path alias**: `@/` → `web/src/`
- **Dark mode**: Tailwind `dark:` variants, controlled by `store.preferColorScheme` (auto/light/dark)
- **No vue-router**: Single-page app, no routes. Sidebar toggles filter panels.
- **Statistics panel**: 13 chart components in `web/src/components/Statistics/`, opened as a modal from Navbar, with independent filter controls (year, R18, AI)

### Statistics module details

- Tauri command: `get_statistics(yearMin, yearMax, r18, isAi)` → `StatisticsResult` with 18 data series
- Backend: `src-tauri/src/stats.rs` — `compute_statistics()` executes 18-19 SQL queries against the filtered dataset
- Frontend: `StatsModal.vue` (modal shell + filter bar) + 13 chart components using Chart.js + vue-chartjs
- Charts: author ranking (horizontal bar + table), tag ranking, yearly trend (bar + cumulative line), bookmark/view distribution, R18/AI pie + sanity bars, shape doughnut, top 10 lists + hidden gems, tag cloud, R18/AI trend, tag stacked area, author discovery timeline
- Filter state is local to StatsModal (not in store), emitted as `applyFilter` event

### pxder (vendored scraper)

- Node.js CLI tool, CommonJS (`"type": "commonjs"` implicitly), no build step
- `pxder/start.bat` chains: scrap bookmark → rename → compress
- Must manually place `ffmpeg.exe` and `node.exe` in `pxder/` for local runs
- Config at `pxder/src/config/config.json` (auto-generated, gitignored)
- Image compression uses `sharp` (WebP) and `ffmpeg` (ugoira→MP4)

### Deployment

- Post-build script `scripts/postbuild.bat`:
  1. Copies exe + pxder/ into `scripts/PixivCollection/`
  2. Cleans pxder/ data, config, node_modules bloat
  3. Compresses to 7z for distribution
- Target: `x86_64-pc-windows-msvc` (Windows only)
- Also supports `build:mac` for universal macOS

### Gotchas

- **No tests anywhere** — manual QA only
- **No CI/CD** — releases built manually
- **yarn v1 required** (classic), not yarn berry
- **Node.js >= 20.16.0** required
- `yarn.lock` at root, but `web/` and `pxder/` have their own deps via `yarn install` at root
- Fancybox CSS/JS loaded from `/fancybox/` — vendor directory not tracked in repo (external)
- ESLint only lints `web/` (config in `.vscode/settings.json`)
- `vue-tsc` type-checks the full frontend — run before any commit
- `cargo check` requires Windows (libsoup-2.4/system deps missing on Linux)
- Chart.js components register their own Chart.js modules (some duplication across components)
- The `_meta` table caches sidebar filter options and full counts — cleared on reimport
