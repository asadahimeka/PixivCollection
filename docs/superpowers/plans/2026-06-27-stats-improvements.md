# Statistics Module — 11-Item Improvements Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement 11 quality-of-life improvements to the statistics modal (StatsModal) covering modal behavior, data display, gallery navigation integration, and word cloud rendering.

**Architecture:** Changes span Rust Tauri backend (`stats.rs`, `main.rs`, `query.rs`) and Vue 3 frontend (13 Statistics components + App.vue). Most changes are independent and can be parallelized. The only dependency chain is: StatsModal (Q2) → App.vue emits (Q4/Q6), but since the emit signatures are defined in advance, files can be edited simultaneously.

**Tech Stack:** Tauri v1, Vue 3 + TS + Vite + Pinia, Chart.js + vue-chartjs, Tailwind CSS, d3-cloud (new dependency), rusqlite

## Global Constraints

- No tests in the codebase — manual QA only
- ESLint: `@antfu/eslint-config-vue` — single quotes, no semicolons, brace-style `1tbs`
- No Prettier; format via ESLint fix
- Auto-imports: Vue APIs + vue-router auto-imported (no manual import of `ref`, `computed`, `reactive`, `watch`, `onMounted`, `onUnmounted`, `nextTick`)
- Path alias: `@/` → `web/src/`
- Rust: `cargo check` requires Windows (libsoup-2.4 missing on Linux) — skip Rust compilation checks
- No new files created except spec/plan docs; all changes modify existing files
- `web/package.json` — add `d3-cloud` and `@types/d3-cloud` dependencies

---
---

### Task 1: Rust stats.rs — TagTrend translated_name + AuthorDiscovery HAVING/ORDER

**Files:**
- Modify: `src-tauri/src/stats.rs`

**Interfaces:**
- Consumes: existing `TagTrendItem` / `AuthorDiscovery` structs and SQL
- Produces: `TagTrendItem` with `translated_name: Option<String>` field; AuthorDiscovery SQL with HAVING/ORDER change

- [ ] **Step 1: Add `translated_name` to `TagTrendItem` struct**

Find the `TagTrendItem` struct definition (~line 149-154) and add the field:

```diff
 /// Tag popularity trend for a single year.
 #[derive(Debug, Serialize)]
 pub struct TagTrendItem {
     pub year: i32,
     pub tag_name: String,
+    pub translated_name: Option<String>,
     pub count: i64,
 }
```

- [ ] **Step 2: Add `t.translated_name` to the tag trend SQL**

Find line ~758 (`let trend_sql = format!(`) and change the SELECT to include `t.translated_name`:

```diff
 let trend_sql = format!(
-    "SELECT t.name as tag_name, \
+    "SELECT t.name as tag_name, t.translated_name, \
             CAST(substr(i.created_at,1,4) AS INTEGER) as year, \
             COUNT(DISTINCT i.id) as count \
```

- [ ] **Step 3: Update the row mapper for TagTrendItem**

Find the `query_map` closure inside the tag trend section (~line 775-781), add `translated_name`:

```diff
 let rows = stmt
     .query_map(rusqlite::params_from_iter(&refs), |row| {
         Ok(TagTrendItem {
             year: row.get("year")?,
             tag_name: row.get("tag_name")?,
+            translated_name: row.get("translated_name")?,
             count: row.get("count")?,
         })
     })
```

- [ ] **Step 4: Change AuthorDiscovery SQL — HAVING + ORDER**

Find line ~811-817:

```diff
     // ---- 17. Author Discovery ----
     let author_discovery_sql = format!(
         "SELECT i.author_id, i.author_name, i.author_account, \
                 CAST(substr(MIN(i.created_at),1,4) AS INTEGER) as first_year, \
                 COUNT(DISTINCT i.id) as works_count \
          FROM images i WHERE {} \
-         GROUP BY i.author_id ORDER BY first_year DESC LIMIT 100",
+         GROUP BY i.author_id \
+         HAVING COUNT(DISTINCT i.id) >= 10 \
+         ORDER BY first_year DESC, works_count DESC",
         where_clause,
     );
```

- [ ] **Step 5: Verify changes are syntactically consistent**

Run: `grep -n 'pub struct TagTrendItem' src-tauri/src/stats.rs`
Run: `grep -n 't.translated_name' src-tauri/src/stats.rs`
Run: `grep -n 'HAVING COUNT' src-tauri/src/stats.rs`
Confirm all three edits are present. (No `cargo check` — requires Windows.)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/stats.rs
git commit -m "feat(stats): add translated_name to TagTrend, filter author discovery by HAVING 10+ works"
```

---
---

### Task 2: Rust main.rs + query.rs — New `get_image_by_id` command

**Files:**
- Modify: `src-tauri/src/main.rs`
- No change needed to `query.rs` — reuses existing `image_row_from_row()` and `ImageRow`/`ImageTag` types

**Interfaces:**
- Produces: New Tauri command `get_image_by_id(id: i64) -> Result<query::ImageRow, String>`
- Later consumed by: App.vue `handleViewArtwork(id)`

- [ ] **Step 1: Add the new `get_image_by_id` command in `main.rs`**

Add after the `get_statistics` function (~line 420-431) and before `fn main()`:

```rust
/// Fetch a single image by its ID (part=0, with tags).
#[tauri::command]
fn get_image_by_id(id: i64) -> Result<query::ImageRow, String> {
    let conn = db::get_conn()?;

    // ---- Query the image ----
    let sql = concat!(
        "SELECT i.id, i.part, i.len, i.title, i.width, i.height, i.ext, ",
        "i.author_id, i.author_name, i.author_account, i.bookmark, i.view, ",
        "i.created_at, i.sanity_level, i.x_restrict, i.is_ai, ",
        "i.img_s, i.img_m, i.img_l, i.img_o FROM images i ",
        "WHERE i.id = ?1 AND i.part = 0",
    );
    let mut img: query::ImageRow = {
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare: {e}"))?;
        stmt.query_row(rusqlite::params![id], query::image_row_from_row)
            .map_err(|e| format!("Query image {id}: {e}"))?
    };

    // ---- Fetch tags (all pages of this id share the same tags) ----
    let tag_sql = concat!(
        "SELECT t.name, t.translated_name FROM tags t ",
        "JOIN image_tags it ON it.tag_id = t.id ",
        "WHERE it.image_id = ?1 AND it.image_part = 0",
    );
    {
        let mut tag_stmt = conn.prepare(&tag_sql).map_err(|e| format!("Tag prepare: {e}"))?;
        let tag_rows = tag_stmt
            .query_map(rusqlite::params![id], |row| {
                Ok(query::ImageTag {
                    name: row.get("name")?,
                    translated_name: row.get("translated_name")?,
                })
            })
            .map_err(|e| format!("Tag query: {e}"))?;
        let mut tags = Vec::new();
        for row in tag_rows {
            tags.push(row.map_err(|e| format!("Tag row: {e}"))?);
        }
        img.tags = tags;
    }

    Ok(img)
}
```

- [ ] **Step 2: Register the command in `generate_handler!`**

Find line ~443-457 where the handler `generate_handler!` lists all commands. Add `get_image_by_id,`:

```diff
         .invoke_handler(tauri::generate_handler![
             start_local_server,
             get_executable_dir,
             restart_app,
             ensure_db,
             query_images,
             query_image_counts,
             get_full_counts,
             refresh_caches,
             reimport_db,
             get_filter_options,
             search_tags,
             import_json_to_db,
             get_statistics,
+            get_image_by_id,
         ])
```

- [ ] **Step 3: Verify the changes**

Run: `grep -n 'fn get_image_by_id' src-tauri/src/main.rs` → confirm exists
Run: `grep -n 'get_image_by_id' src-tauri/src/main.rs | grep generate_handler` → confirm registered

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat(backend): add get_image_by_id Tauri command for single image fetch"
```

---
---

### Task 3: StatsModal.vue — v-show, full-size, new emits

**Files:**
- Modify: `web/src/components/Statistics/StatsModal.vue`
- No interface dependency: new emits defined here, consumed by App.vue later

- [ ] **Step 1: Change `v-if="show"` to `v-show="show"` on the inner div**

Line 4: `v-if="show"` → `v-show="show"`. Keep the `<Transition name="fade">` wrapper as-is.

```diff
-     v-if="show"
+     v-show="show"
```

- [ ] **Step 2: Full-size modal — change overlay class**

Line 5:
```diff
-      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
+      class="fixed inset-0 z-50 flex bg-black/60"
```

- [ ] **Step 3: Full-size modal — change inner container class**

Line 9:
```diff
-        class="relative flex max-h-[90vh] w-full max-w-[90vw] flex-col rounded-2xl bg-white shadow-2xl dark:bg-[#242424] dark:text-white"
+        class="relative flex h-full w-full flex-col bg-white dark:bg-[#242424] dark:text-white"
```

- [ ] **Step 4: Add new emits to the defineEmits**

Line 163-167:
```diff
 const emit = defineEmits<{
   close: []
   'applyFilter': [filter: StatsFilter]
   retry: []
+  viewAuthor: [id: number]
+  viewTag: [name: string]
 }>()
```

- [ ] **Step 5: Verify changes**

Read the file header to confirm the three class changes and the emit additions are in place.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Statistics/StatsModal.vue
git commit -m "feat(stats): v-show, full-size modal, add viewAuthor/viewTag emits"
```

---
---

### Task 4: StatsAuthorChart.vue — Q4 emit, Q8 placeholder, Q9 un-reverse

**Files:**
- Modify: `web/src/components/Statistics/StatsAuthorChart.vue`

- [ ] **Step 1: Add `viewAuthor` emit to the component**

After the existing `defineProps` (~line 112), add:
```ts
const emit = defineEmits<{
  viewAuthor: [id: number]
}>()
```

- [ ] **Step 2: Change author `<a>` link to clickable `<span>` in table**

Lines 77-85:
```diff
-                <a
-                  :href="userLink(author.author_id)"
-                  target="_blank"
-                  rel="noopener noreferrer"
-                  class="block truncate text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
-                  :title="author.author_name"
-                >
-                  {{ author.author_name }}
-                </a>
+                <span
+                  class="block truncate text-blue-600 cursor-pointer transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
+                  :title="author.author_name"
+                  @click="emit('viewAuthor', author.author_id)"
+                >
+                  {{ author.author_name || '(佚名)' }}
+                </span>
```

- [ ] **Step 3: Add empty name fallback in tooltip label**

Line ~211 (tooltip `label` callback):
```diff
         label(ctx: any) {
-          return `作品数: ${ctx.parsed.x}`
+          const name = ctx.chart.data.labels[ctx.dataIndex] || '(佚名)'
+          return `${name}: ${ctx.parsed.x}`
         },
```

- [ ] **Step 4: Un-reverse the chart data — largest on top**

Lines 168-173:
```diff
 const chartData = computed(() => {
-  const reversed = [...top20.value].reverse()
   const colors = generateGradientColors(20)
   const label = sortMode.value === 'illustration_count' ? '作品数' : '总收藏'
   return {
-    labels: reversed.map(a => a.author_name),
+    labels: top20.value.map(a => a.author_name),
     datasets: [
       {
         label,
-        data: reversed.map(a => a[sortMode.value]),
+        data: top20.value.map(a => a[sortMode.value]),
```

- [ ] **Step 5: Clean up unused imports**

Line 108: Remove `import { LINK_PIXIV_USER } from '@/config'` since `userLink()` is no longer used (the function can be removed too, or kept — the emit-based span doesn't need it).

Also remove the `userLink` function (~line 143-145):
```diff
-function userLink(id: number): string {
-  return LINK_PIXIV_USER.replace('{id}', String(id))
-}
```

- [ ] **Step 6: Verify**

Read file to confirm: emit added, `<a>`→`<span>` changed, `.reverse()` removed, `userLink` removed.

- [ ] **Step 7: Commit**

```bash
git add web/src/components/Statistics/StatsAuthorChart.vue
git commit -m "feat(stats): author chart viewAuthor emit, placeholder, un-reverse bars"
```

---
---

### Task 5: StatsAuthorDiscovery.vue — Q4 emit, Q8 placeholder, Q12 per-year cap

**Files:**
- Modify: `web/src/components/Statistics/StatsAuthorDiscovery.vue`

- [ ] **Step 1: Add `viewAuthor` emit**

After `defineProps` (~line 122-124), add:
```ts
const emit = defineEmits<{
  viewAuthor: [id: number]
}>()
```

- [ ] **Step 2: Change author `<a>` link to clickable `<span>` in timeline**

Lines 79-98:
```diff
-          <a
-            v-for="author in group.authors"
-            :key="author.author_id"
-            :href="userLink(author.author_id)"
-            target="_blank"
-            rel="noopener noreferrer"
-            class="group flex items-center justify-between rounded-lg px-3 py-2.5 transition-colors hover:bg-blue-50/60 dark:hover:bg-blue-900/10"
-          >
-            <div class="flex min-w-0 items-center gap-2">
-              <span class="truncate text-sm font-medium text-blue-600 transition-colors group-hover:text-blue-800 dark:text-blue-400 dark:group-hover:text-blue-300">
-                {{ author.author_name }}
-              </span>
-              <span class="shrink-0 text-xs text-gray-400 dark:text-gray-500">
-                @{{ author.author_account }}
-              </span>
-            </div>
-            <span class="ml-2 shrink-0 whitespace-nowrap text-xs text-gray-500 dark:text-gray-400">
-              作品数 {{ author.works_count }} 件
-            </span>
-          </a>
+          <div
+            v-for="author in group.authors"
+            :key="author.author_id"
+            class="group flex cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 transition-colors hover:bg-blue-50/60 dark:hover:bg-blue-900/10"
+            @click="emit('viewAuthor', author.author_id)"
+          >
+            <div class="flex min-w-0 items-center gap-2">
+              <span class="truncate text-sm font-medium text-blue-600 transition-colors group-hover:text-blue-800 dark:text-blue-400 dark:group-hover:text-blue-300">
+                {{ author.author_name || '(佚名)' }}
+              </span>
+              <span class="shrink-0 text-xs text-gray-400 dark:text-gray-500">
+                @{{ author.author_account }}
+              </span>
+            </div>
+            <span class="ml-2 shrink-0 whitespace-nowrap text-xs text-gray-500 dark:text-gray-400">
+              作品数 {{ author.works_count }} 件
+            </span>
+          </div>
```

- [ ] **Step 3: Add per-year cap of 100 authors in `groups` computed**

Lines 131-143:
```diff
 const groups = computed<YearGroup[]>(() => {
   const map = new Map<number, AuthorDiscovery[]>()
   for (const author of props.authorDiscovery) {
     const list = map.get(author.first_year)
     if (list) {
       list.push(author)
     } else {
       map.set(author.first_year, [author])
     }
   }
   return Array.from(map.entries())
-    .map(([year, authors]) => ({ year, authors }))
+    .map(([year, authors]) => ({ year, authors: authors.slice(0, 100) }))
     .sort((a, b) => b.year - a.year)
 })
```

- [ ] **Step 4: Clean up unused imports/functions**

Remove `import { LINK_PIXIV_USER } from '@/config'` (line 107) and the `userLink` function (lines 153-155).

- [ ] **Step 5: Verify**

Read file to confirm: emit added, `<a>`→`<div @click>` changed, `.slice(0, 100)` added, `userLink` removed.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Statistics/StatsAuthorDiscovery.vue
git commit -m "feat(stats): author discovery click emit, placeholder, per-year 100 cap"
```

---
---

### Task 6: StatsTagChart.vue — Q5 two-column table, Q6 emit, Q9 un-reverse

**Files:**
- Modify: `web/src/components/Statistics/StatsTagChart.vue`

- [ ] **Step 1: Add `viewTag` emit**

After `defineProps` (~line 75-77), add:
```ts
const emit = defineEmits<{
  viewTag: [name: string]
}>()
```

- [ ] **Step 2: Chart labels — show original name, un-reverse**

Lines 92-96:
```diff
 const chartData = computed(() => {
   const top = props.tags.slice(0, 30)
-  // reverse so highest count is at top of horizontal bar chart
-  const labels = top.map(t => t.translated_name || t.name).reverse()
-  const counts = top.map(t => t.count).reverse()
+  const labels = top.map(t => t.name)
+  const counts = top.map(t => t.count)

```

- [ ] **Step 3: Table header — change to two columns**

Lines 29-35:
```diff
               <th class="sticky top-0 z-10 bg-white px-4 py-2.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:bg-[#242424] dark:text-gray-400">
                 排名
               </th>
               <th class="sticky top-0 z-10 bg-white px-4 py-2.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:bg-[#242424] dark:text-gray-400">
-                 标签名
+                 原名
+               </th>
+               <th class="sticky top-0 z-10 bg-white px-4 py-2.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:bg-[#242424] dark:text-gray-400">
+                 译名
               </th>
```

- [ ] **Step 4: Table cell — split into two columns (name + translated_name), add click**

Lines 48-49:
```diff
-              <td class="px-4 py-2 text-gray-900 dark:text-gray-100">
-                {{ tag.translated_name || tag.name }}
+              <td
+                class="cursor-pointer px-4 py-2 text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
+                @click="emit('viewTag', tag.name)"
+              >
+                {{ tag.name }}
+              </td>
+              <td class="px-4 py-2 text-gray-500 dark:text-gray-400">
+                {{ tag.translated_name || '-' }}
               </td>
```

Also add `colspan="3"` to the empty state row (line 57) → `colspan="4"` since we went from 3 columns to 4.

```diff
-                colspan="3"
+                colspan="4"
```

- [ ] **Step 5: Verify**

Read file to confirm: emit added, chart labels use `t.name`, table has two name columns, `.reverse()` removed, click handler on tag name cell.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Statistics/StatsTagChart.vue
git commit -m "feat(stats): tag chart show original name, two-column table, click emit, un-reverse"
```

---
---

### Task 7: StatsTopLists.vue — Q8 placeholder, Q10 emit viewArtwork

**Files:**
- Modify: `web/src/components/Statistics/StatsTopLists.vue`

- [ ] **Step 1: Add `viewArtwork` emit**

After `defineProps` (~line 237-241), add:
```ts
const emit = defineEmits<{
  viewArtwork: [id: number]
}>()
```

- [ ] **Step 2: Add empty author placeholder in all three tables**

Line 59: `{{ item.author_name }}` → `{{ item.author_name || '(佚名)' }}`
Line 126: `{{ item.author_name }}` → `{{ item.author_name || '(佚名)' }}`
Line 201: `{{ item.author_name }}` → `{{ item.author_name || '(佚名)' }}`

- [ ] **Step 3: Change artwork link behavior — emit instead of window.open**

Lines 46-56 (top bookmarked table):
```diff
               <td class="max-w-0 px-3 py-2">
-                <a
-                  :href="artworkLink(item.id)"
-                  target="_blank"
-                  rel="noopener noreferrer"
-                  class="block truncate text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
-                  :title="item.title"
-                  @click.prevent="openArtwork(item.id)"
-                >
-                  {{ item.title }}
-                </a>
+                <span
+                  class="block cursor-pointer truncate text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
+                  :title="item.title"
+                  @click="emit('viewArtwork', item.id)"
+                >
+                  {{ item.title }}
+                </span>
```

Same change for lines 113-122 (top viewed table) and lines 187-196 (hidden gems table).
Each: `<a href="..." target="_blank" @click.prevent="openArtwork(...)">` → `<span @click="emit('viewArtwork', item.id)">`.

- [ ] **Step 4: Remove unused imports and functions**

Remove `import { LINK_PIXIV_ARTWORK } from '@/config'` (line 219).
Remove `artworkLink()` function (lines 257-259).
Remove `openArtwork()` function (lines 261-263).

- [ ] **Step 5: Verify**

Read file to confirm: emit added, author names have `|| '(佚名)'`, artwork titles use `<span @click>` emit, unused code removed.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Statistics/StatsTopLists.vue
git commit -m "feat(stats): top lists viewArtwork emit, author placeholder, remove external links"
```

---
---

### Task 8: StatsWordCloud.vue — Q11 d3-cloud rewrite + Q6 emit

**Files:**
- Modify: `web/src/components/Statistics/StatsWordCloud.vue`
- Modify: `web/package.json` (add dependencies)

- [ ] **Step 1: Install d3-cloud dependencies**

```bash
cd web && yarn add d3-cloud && yarn add -D @types/d3-cloud
```

- [ ] **Step 2: Add `viewTag` emit**

After `defineProps` (~line 41-43), add:
```ts
const emit = defineEmits<{
  viewTag: [name: string]
}>()
```

- [ ] **Step 3: Rewrite the template — replace flex-wrap with SVG**

Replace the entire `<template>` content:

```diff
 <template>
   <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
     <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
       标签词云
     </h3>
     <div
-      v-if="displayTags.length > 0"
-      class="flex flex-wrap items-center justify-center gap-2 p-4"
+      v-if="layoutWords.length > 0"
+      ref="containerRef"
+      class="relative flex items-center justify-center"
+      :style="{ height: containerHeight + 'px' }"
     >
-      <span
-        v-for="(tag, index) in displayTags"
-        :key="tag.name"
-        :title="`${tag.translated_name || tag.name} (${tag.count})`"
-        :style="{
-          fontSize: `${fontSize(tag.count)}px`,
-          color: tagColor(index),
-        }"
-        class="cursor-pointer transition-all duration-200 hover:scale-110 hover:opacity-80"
-      >
-        {{ tag.translated_name || tag.name }}
-      </span>
+      <svg
+        v-if="svgSize.width > 0"
+        :width="svgSize.width"
+        :height="svgSize.height"
+        class="overflow-visible"
+      >
+        <text
+          v-for="w in layoutWords"
+          :key="w.text"
+          :x="w.x"
+          :y="w.y"
+          :font-size="w.size"
+          :fill="w.color"
+          :transform="`rotate(${w.rotate}, ${w.x}, ${w.y})`"
+          text-anchor="middle"
+          class="cursor-pointer transition-opacity hover:opacity-70"
+          :style="{ fontFamily: 'system-ui, sans-serif' }"
+          @click="emit('viewTag', w.text)"
+        >
+          <title>{{ displayTagMap[w.text]?.translated_name || w.text }} ({{ displayTagMap[w.text]?.count || 0 }})</title>
+          {{ w.text }}
+        </text>
+      </svg>
     </div>
     <div
-      v-else
+      v-if="displayTags.length === 0"
       class="flex items-center justify-center py-12 text-sm text-gray-400 dark:text-gray-500"
     >
       暂无标签数据
     </div>
   </div>
 </template>
```

- [ ] **Step 4: Rewrite the script — d3-cloud layout logic**

Replace the entire `<script setup>` with:

```ts
<script setup lang="ts">
import cloud from 'd3-cloud'
import { useStore } from '@/store'

interface TagStats {
  name: string
  translated_name: string | null
  count: number
}

const props = defineProps<{
  tags: TagStats[]
}>()

const emit = defineEmits<{
  viewTag: [name: string]
}>()

const store = useStore()
const isDark = computed(() => store.colorScheme === 'dark')
const containerRef = ref<HTMLElement>()
const containerHeight = ref(400)

const COLOR_PALETTE_LIGHT = [
  '#2563eb', '#16a34a', '#9333ea', '#ea580c', '#0d9488',
  '#db2777', '#4f46e5', '#dc2626', '#0891b2', '#ca8a04',
]
const COLOR_PALETTE_DARK = [
  '#60a5fa', '#4ade80', '#c084fc', '#fb923c', '#2dd4bf',
  '#f472b6', '#818cf8', '#f87171', '#22d3ee', '#facc15',
]

const displayTags = computed(() => {
  return [...props.tags]
    .sort((a, b) => b.count - a.count)
    .slice(0, 100)
})

// Map for lookup in template title
const displayTagMap = computed(() => {
  const map: Record<string, TagStats> = {}
  for (const t of displayTags.value) {
    map[t.name] = t
  }
  return map
})

const minCount = computed(() => {
  if (displayTags.value.length === 0) return 0
  return displayTags.value[displayTags.value.length - 1].count
})
const maxCount = computed(() => {
  if (displayTags.value.length === 0) return 0
  return displayTags.value[0].count
})

function fontSize(count: number): number {
  if (maxCount.value === minCount.value) return 30
  const ratio = (count - minCount.value) / (maxCount.value - minCount.value)
  return Math.round(14 + ratio * 40)
}

function tagColor(index: number): string {
  const palette = isDark.value ? COLOR_PALETTE_DARK : COLOR_PALETTE_LIGHT
  return palette[index % palette.length]
}

// Cloud layout state
const layoutWords = ref<cloud.Word[]>([])
const svgSize = reactive({ width: 0, height: 0 })

function computeSvgSize() {
  if (!containerRef.value) return
  const rect = containerRef.value.getBoundingClientRect()
  const w = Math.max(rect.width - 20, 300)
  const h = Math.min(Math.max(w * 0.5, 300), 500)
  svgSize.width = w
  svgSize.height = h
  containerHeight.value = h
}

function layoutCloud() {
  if (displayTags.value.length === 0) {
    layoutWords.value = []
    return
  }

  computeSvgSize()
  const { width, height } = svgSize
  if (width === 0 || height === 0) return

  const words = displayTags.value.map((t, i) => ({
    text: t.name,
    size: fontSize(t.count),
    color: tagColor(i),
  }))

  cloud()
    .size([width, height])
    .words(words)
    .padding(3)
    .rotate(() => (Math.random() > 0.5 ? 0 : 90))
    .font('system-ui, sans-serif')
    .spiral('archimedean')
    .on('end', (placed: cloud.Word[]) => {
      layoutWords.value = placed
    })
    .start()
}

watch(displayTags, () => {
  nextTick(() => layoutCloud())
}, { deep: true })

watch(isDark, () => {
  nextTick(() => layoutCloud())
})

onMounted(() => {
  const observer = new ResizeObserver(() => {
    layoutCloud()
  })
  if (containerRef.value) {
    observer.observe(containerRef.value)
  }
  onUnmounted(() => observer.disconnect())
  nextTick(() => layoutCloud())
})
</script>
```

- [ ] **Step 5: Verify**

Read the file to confirm: template uses SVG, d3-cloud imported, emit defined, `displayTagMap` used for hover titles.

- [ ] **Step 6: Commit**

```bash
git add web/package.json web/yarn.lock web/src/components/Statistics/StatsWordCloud.vue
git commit -m "feat(stats): d3-cloud word cloud, viewTag emit, SVG layout"
```

---
---

### Task 9: StatsTagTrend.vue — Q5 +translated_name

**Files:**
- Modify: `web/src/components/Statistics/StatsTagTrend.vue`

- [ ] **Step 1: Update the `TagTrendItem` interface to add `translated_name`**

Lines 40-44:
```diff
 interface TagTrendItem {
   year: number
   tag_name: string
+  translated_name?: string | null
   count: number
 }
```

- [ ] **Step 2: Update the legend toggle buttons to show translated name on hover**

Lines 14-25 (the `<button>` in the legend):
```diff
       <button
         v-for="(tag, i) in topTags"
         :key="tag"
         class="flex items-center gap-1.5 text-xs transition-opacity hover:opacity-80"
         :class="hiddenTags.has(tag) ? 'opacity-40' : 'opacity-90'"
         @click="toggleTag(tag)"
+        :title="tagTranslatedName(tag)"
       >
```

- [ ] **Step 3: Add a helper to look up translated_name**

Add after the `toggleTag` function (~line 180-185):
```ts
function tagTranslatedName(name: string): string | undefined {
  const item = props.tagTrend.find(d => d.tag_name === name)
  return item?.translated_name ?? undefined
}
```

- [ ] **Step 4: Verify**

Read file to confirm: interface updated, `:title` added to legend buttons, helper function added.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/Statistics/StatsTagTrend.vue
git commit -m "feat(stats): tag trend show translated_name on hover"
```

---
---

### Task 10: App.vue — wire all handlers, fetch guard

**Files:**
- Modify: `web/src/App.vue`

- [ ] **Step 1: Add `TagTrendItemData` interface update for translated_name**

After ~line 235, update the interface:
```diff
-interface TagTrendItemData { year: number; tag_name: string; count: number }
+interface TagTrendItemData { year: number; tag_name: string; translated_name?: string | null; count: number }
```

- [ ] **Step 2: Guard `fetchStatistics()` to not re-fetch if data exists (unless filter changed)**

Lines 265-283:
```diff
-async function fetchStatistics(filter?: { year_min?: number | null; year_max?: number | null; r18?: string | null; is_ai?: boolean | null }) {
+async function fetchStatistics(filter?: { year_min?: number | null; year_max?: number | null; r18?: string | null; is_ai?: boolean | null }, force = false) {
+  // Skip fetch when data already cached, unless forced (e.g. filter changed)
+  if (!force && statsData.value !== null && !filter) return
+
   statsLoading.value = true
   statsError.value = null
```

- [ ] **Step 3: Update the `@open-stats` handler to pass `force`**

Line 6:
```diff
-<Navbar @updatebookmark="updateBookmark()" @open-stats="showStatsModal = true; fetchStatistics()" />
+<Navbar @updatebookmark="updateBookmark()" @open-stats="showStatsModal = true; statsData.value === null && fetchStatistics()" />
```

- [ ] **Step 4: Update `@apply-filter` to force fetch**

Line 86:
```diff
-@apply-filter="fetchStatistics"
+@apply-filter="fetchStatistics($event, true)"
```

- [ ] **Step 5: Wire handler methods on StatsModal slot content**

Replace the entire `<StatsModal>` block (lines 80-147) to pass new emit handlers. The key changes are:

Add `@view-author`, `@view-tag`, and `@view-artwork` events to the child component wrappers. Since the slot content is inside StatsModal which receives these emits via its slot, we need to use a different approach: **StatsModal proxies the child emits upward via its own emits** declared in Task 3. Then in App.vue we bind:

```diff
       <StatsModal
         :show="showStatsModal"
         :stats="statsData"
         :loading="statsLoading"
         :error="statsError"
         @close="showStatsModal = false"
-        @apply-filter="fetchStatistics"
+        @apply-filter="fetchStatistics($event, true)"
         @retry="fetchStatistics"
+        @view-author="handleViewAuthor"
+        @view-tag="handleViewTag"
       >
```

But wait — the child components (StatsAuthorChart, StatsTagChart, etc.) are inside the slot of StatsModal. Their emits don't automatically bubble through StatsModal. We need another approach.

**Solution**: StatsModal as a shell doesn't pass through emits from slot children. Instead, each child component should emit events that StatsModal catches and re-emits, OR we put the handlers directly on the child components in the slot.

Actually, looking at the current code in App.vue, the child components are rendered directly in the slot:

```html
<StatsModal ...>
  <section><StatsAuthorChart :authors="..." /></section>
  ...
</StatsModal>
```

StatsAuthorChart can emit `viewAuthor` but StatsModal doesn't know about it. The simplest approach: **put the handler directly on StatsAuthorChart in App.vue** using `@view-author`. But `@view-author` on a custom component requires the component to emit it.

Wait — actually Vue allows `@view-author` on a component if the component defines the emit. The emit will bubble up to the parent (App.vue) because the event is emitted from the child. In Vue 3, `emit('viewAuthor', id)` from inside StatsAuthorChart will be caught by `@view-author` on the `<StatsAuthorChart>` tag in App.vue, regardless of whether StatsModal is in between (since StatsModal uses slot, not wrapping the children).

Actually, that's not how it works. The emit chain is: StatsAuthorChart emits → caught by whoever has `@view-author` on the **StatsAuthorChart** element. Since StatsAuthorChart appears inside the StatsModal slot, and the slot content is rendered in App.vue's template scope, `@view-author` on `<StatsAuthorChart>` in App.vue will work.

Let me add the handlers directly on each child component in the slot.

```diff
           <section>
-            <StatsAuthorChart :authors="statsData.author_ranking" />
+            <StatsAuthorChart
+              :authors="statsData.author_ranking"
+              @view-author="handleViewAuthor"
+            />
           </section>
           <section>
-            <StatsTagChart :tags="statsData.tag_ranking" />
+            <StatsTagChart
+              :tags="statsData.tag_ranking"
+              @view-tag="handleViewTag"
+            />
           </section>
```

And for StatsTopLists, StatsWordCloud, StatsAuthorDiscovery — same pattern.

- [ ] **Step 6: Add handler functions before `fetchStatistics`**

Add after the `statsError` declaration (~line 263):
```ts
function handleViewAuthor(id: number) {
  store.filterConfig.author.id = id
  store.filterConfig.author.enable = true
  store.sortImages()
  showStatsModal.value = false
}

function handleViewTag(name: string) {
  store.filterConfig.tag.name = name
  store.filterConfig.tag.enable = true
  // Preserve existing tag filter config defaults
  store.sortImages()
  showStatsModal.value = false
}

async function handleViewArtwork(id: number) {
  try {
    const row = await invoke<any>('get_image_by_id', { id })
    const img: Image = {
      id: row.id,
      part: row.part,
      len: row.len,
      title: row.title,
      ext: row.ext,
      size: [row.width, row.height] as [number, number],
      author: { id: row.author_id, name: row.author_name, account: row.author_account },
      tags: row.tags ?? [],
      created_at: row.created_at,
      sanity_level: row.sanity_level,
      x_restrict: row.x_restrict,
      dominant_color: '',
      bookmark: row.bookmark,
      view: row.view,
      images: { s: row.img_s, m: row.img_m, l: row.img_l, o: row.img_o },
      isAI: row.is_ai,
    }
    store.openImageViewer(img, () => {}, () => {}, -1)
  } catch (e) {
    console.error('Failed to open artwork:', e)
  }
}
```

- [ ] **Step 7: Wire @view-artwork on StatsTopLists**

Find the StatsTopLists section in the template:
```diff
             <StatsTopLists
               :top-bookmarked="statsData.top_bookmarked"
               :top-viewed="statsData.top_viewed"
               :hidden-gems="statsData.hidden_gems"
+              @view-artwork="handleViewArtwork"
             />
```

--- Wait, StatsTopLists emits `viewArtwork` but the emit name in the component is `viewArtwork`. In Vue templates, camelCase emits are automatically converted to kebab-case for template binding... actually no — in Vue 3, both `@view-artwork` and `@viewArtwork` work. Let me use the convention in the codebase. Looking at existing code, there's `@open-stats` and `@apply-filter`. So let me use `@view-author`, `@view-tag`, `@view-artwork`.

But the emit defined in StatsTopLists is `viewArtwork`. In Vue 3, `emit('viewArtwork')` can be caught as `@view-artwork` or `@viewArtwork`. The existing code uses:
- `@updatebookmark` (emit `updatebookmark`)
- `@open-stats` (emit `open-stats`)
- `@apply-filter` (emit `applyFilter`)

So the codebase uses kebab-case for template events. Let me use `@view-artwork`, `@view-author`, `@view-tag`.

- [ ] **Step 8: Wire all @view-* handlers in the template**

```diff
             <section>
-              <StatsWordCloud :tags="statsData.tag_ranking" />
+              <StatsWordCloud
+                :tags="statsData.tag_ranking"
+                @view-tag="handleViewTag"
+              />
             </section>
             <section>
-              <StatsAuthorDiscovery :author-discovery="statsData.author_discovery" />
+              <StatsAuthorDiscovery
+                :author-discovery="statsData.author_discovery"
+                @view-author="handleViewAuthor"
+              />
             </section>
```

- [ ] **Step 9: Remove unused StatsModal emit bindings (since child emits are now directly on children)**

Remove `@view-author` and `@view-tag` from `<StatsModal>` if added — they should NOT be on StatsModal, they should be on the **child components**.

- [ ] **Step 10: Verify**

Read App.vue to confirm: all handlers added, child components have `@view-*` bindings, fetchStatistics has guard.

- [ ] **Step 11: Commit**

```bash
git add web/src/App.vue
git commit -m "feat(stats): wire all view handlers, fetchStatistics cache guard"
```

---
---

## Self-Review Checklist

- [ ] **Spec coverage**: Every item Q2-Q12 has a corresponding task with implementation steps
- [ ] **Placeholder scan**: No "TBD", "TODO", or vague steps — all code changes are explicit
- [ ] **Type consistency**: All interfaces (TagTrendItemData, emits) are consistent between consumer and producer tasks
- [ ] **File paths**: Every file path is exact and verified against the codebase
- [ ] **No new files**: All changes modify existing files only (plus package.json dependency addition)
- [ ] **Import cleanup**: Old `LINK_PIXIV_USER`, `LINK_PIXIV_ARTWORK` imports and `userLink`/`artworkLink`/`openArtwork` functions removed from components that no longer use them
