# AGENTS.md — PixivCollection

## Project overview

A Tauri v1 desktop app that scrapes Pixiv bookmarks locally, converts images to WebP and ugoira to MP4, and provides a local gallery browser. No backend — all data lives on the filesystem or is fetched from Pixiv API proxies.

Three packages in one monorepo:

| Directory | Role | Stack |
|-----------|------|-------|
| `web/` | Desktop UI (Vue 3 SPA) | Vue 3 + TS + Vite + Pinia + Tailwind CSS |
| `src-tauri/` | Tauri Rust backend | Tauri v1 + warp (local HTTP server) |
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

### Data flow (local mode)
1. User sets RefreshToken + storage path → saved to `pxder/src/config/config.json`
2. "Update bookmark" spawns `cmd /C start.bat` in pxder/ directory
3. start.bat: `pxder --debug -b` (scrape) → `rename.mjs` (build `data/images.json`) → `compress.mjs` (sharp: images→webp, ffmpeg: zip→mp4)
4. App reads `images.json` and shows gallery

### Data flow (remote API mode)
- Set only userId → fetches via `https://hibiapi.cocomi.eu.org/api/pixiv/favorite?id=...`
- Image proxy replacement: `i.pximg.net` → `pximg.cocomi.eu.org`

### Image data
- Full image list stored on `window.__fullImages__` (not in Pinia store)
- Filtered/paginated view on `store.imagesFiltered`
- Each image enriched with `part`/`len` fields (multi-page illustrations flattened to one entry per page)
- Locally cached images named: `({id}){title}_p{part}.webp` (or `.mp4` for ugoira)
- Path: `{imgDir}/bookmark_webp/`, `{imgDir}/bookmark_ugoira/`

### Config storage
- `@orilight/vue-settings` (prefix `PXCT`) persists to localStorage automatically
- Settings registered in `App.vue` `onMounted` — always register new settings there
- Window `__CONFIG__` injected at HTML load from localStorage keys (`__PXCT_*`)

### State management (Pinia store)
- Single store at `web/src/store/index.ts`
- Store holds: filter config, masonry config, color scheme, viewer state, image counts
- Types in `web/src/types/index.d.ts` (global `Image`, `Tag`, etc.)
- Config constants in `web/src/config/index.ts`

### Local HTTP server
- Tauri command `start_local_server` runs warp on `127.0.0.1:32154`
- Serves static files from the configured image directory
- Only starts once per session (guarded by `sessionStorage`)

### Tauri specifics (v1)
- `withGlobalTauri: true` → access via `window.__TAURI__` or top-level `__TAURI__`
- Allowlist: all APIs enabled (fs, path, shell, http, protocol)
- Window: 1280×800 default, min 375×600, centered
- Plugins: `window-state` (persists window position/size), `single-instance`
- Commands: `start_local_server`, `get_executable_dir`, `restart_app`

### Frontend conventions
- **ESLint**: `@antfu/eslint-config-vue` — single quotes, no semicolons, brace-style `1tbs`
- **Formatting**: ESLint fix on save via VSCode, no Prettier
- **Auto-imports**: Vue APIs + vue-router auto-imported (see `auto-imports.d.ts`)
- **Component auto-registration**: `unplugin-vue-components` — use PascalCase in templates directly
- **Path alias**: `@/` → `web/src/`
- **Dark mode**: Tailwind `dark:` variants, controlled by `store.preferColorScheme` (auto/light/dark)
- **No vue-router**: Single-page app, no routes. Sidebar toggles filter panels.

### pxder (vendored scraper)
- Node.js CLI tool, CommonJS (`"type": "commonjs"` implicitly), no build step
- `pxder/start.bat` chains: scrap bookmark → rename → compress
- Must manually place `ffmpeg.exe` and `node.exe` in `pxder/` for local runs
- Config at `pxder/src/config/config.json` (auto-generated, gitignored-ish)
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
- Image sort is in-memory only (`imageSortBy` in store), not persisted to `images.json`
- Filter operates on the sorted full image list — performance degrades with 10k+ images
- ESLint only lints `web/` (config in `.vscode/settings.json`)
