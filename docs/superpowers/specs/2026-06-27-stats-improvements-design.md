# Statistics Module — 11-Item Improvement Spec

**Date**: 2026-06-27  
**Project**: PixivCollection (Tauri v1 + Vue 3 + Tailwind CSS)  
**Scope**: Changes to the statistics modal (StatsModal) and 7 child components + Rust backend

---

## 1. Modal lifecycle: `v-show` + data cache (Q2)

**Files**: `web/src/components/Statistics/StatsModal.vue`, `web/src/App.vue`

| Change | Before | After |
|--------|--------|-------|
| DOM mount | `v-if="show"` | `v-show="show"` (keep `<Transition name="fade">` wrapping container for animation) |
| Data fetch | called every open | `fetchStatistics()` guards with `if (statsData.value === null)` |
| Data clear on close | `statsData.value = null` | kept (not cleared) |

**Details**:
- StatsModal: `v-if` on the inner `<div>` → `v-show`, keep the outer `<Transition name="fade">` so entry/exit animation still works
- App.vue: `fetchStatistics()` wraps content with `if (!statsData.value)` so only null-triggered. The `applyFilter` path always passes new filter → bypasses guard (filter changed → need new data)
- Modal stays `v-show` = DOM preserved → scroll position, filter bar selections all survive close/reopen

---

## 2. Full-size modal, no backdrop blur (Q7)

**File**: `web/src/components/Statistics/StatsModal.vue`

```diff
- class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
+ class="fixed inset-0 z-50 flex bg-black/60"
- class="relative flex max-h-[90vh] w-full max-w-[90vw] flex-col rounded-2xl bg-white shadow-2xl dark:bg-[#242424] dark:text-white"
+ class="relative flex h-full w-full flex-col bg-white dark:bg-[#242424] dark:text-white"
```

- Inner container fills screen (`h-full w-full`)
- No rounded corners, no max-height/max-width constraints
- Header bar + filter bar + scrollable content area unchanged

---

## 3. Author click → gallery filter (Q4)

**Files**: `App.vue`, `StatsModal.vue`, `StatsAuthorChart.vue`, `StatsAuthorDiscovery.vue`

### Component changes

**StatsAuthorChart.vue** — table `<a>` author link → clickable `<span>`:
```diff
- <a :href="userLink(author.author_id)" target="_blank" ...>
-   {{ author.author_name }}
- </a>
+ <span
+   class="block truncate text-blue-600 cursor-pointer transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
+   :title="author.author_name"
+   @click="emit('viewAuthor', author.author_id)"
+ >
+   {{ author.author_name || '(佚名)' }}
+ </span>
```
Add `defineEmits<{ viewAuthor: [id: number] }>()`.

**StatsAuthorDiscovery.vue** — same pattern: `<a href>` → `<span @click>`.

**StatsModal.vue** — proxy child emits to parent:
```ts
defineEmits<{
  close: []
  applyFilter: [filter: StatsFilter]
  retry: []
  viewAuthor: [id: number]   // new
  viewTag: [name: string]    // new
}>()
```
In template: pass `@view-author` or aggregate via slot.

### App.vue

New handler:
```ts
function handleViewAuthor(id: number) {
  store.filterConfig.author.id = id
  store.filterConfig.author.enable = true
  store.sortImages()
  showStatsModal.value = false
}
```
StatsModal slot emits `view-author` → bind to handler.

---

## 4. Tag click → gallery filter (Q6)

**Files**: `App.vue`, `StatsModal.vue`, `StatsTagChart.vue`, `StatsWordCloud.vue`

Same pattern as Q4:
- `StatsTagChart.vue` — table `<td>` tag name → `<span @click="emit('viewTag', tag.name)">` with `cursor-pointer`
- `StatsWordCloud.vue` — word cloud `<span>` → add click emit
- StatsModal proxy → App.vue handler:

```ts
function handleViewTag(name: string) {
  store.filterConfig.tag.name = name
  store.filterConfig.tag.enable = true
  store.sortImages()
  showStatsModal.value = false
}
```

---

## 5. Tag original + translated name display (Q5)

**Files**: `StatsTagChart.vue`, `StatsWordCloud.vue`, `StatsTagTrend.vue`, `src-tauri/src/stats.rs`

### Chart display

| Component | Change |
|-----------|--------|
| StatsTagChart bar chart labels | `.map(t => t.translated_name \|\| t.name)` → `.map(t => t.name)` |
| StatsWordCloud | `{{ tag.translated_name \|\| tag.name }}` → `{{ tag.name }}`, hover title stays as translation |
| StatsTagTrend legend | Keep `tag.tag_name`, add hover tooltip showing translated name |

### Table display (StatsTagChart)

One-column "标签名" → two columns: "原名" (tag.name) + "译名" (tag.translated_name ?? '-')
```html
<th>标签名</th> → <th>原名</th> <th>译名</th>
<td>{{ tag.translated_name || tag.name }}</td> → 
  <td>{{ tag.name }}</td> <td>{{ tag.translated_name || '-' }}</td>
```

### Rust backend (TagTrend)

`TagTrendItem` struct:
```diff
 pub struct TagTrendItem {
     pub year: i32,
     pub tag_name: String,
+    pub translated_name: Option<String>,
     pub count: i64,
 }
```

Tag trend SQL: add `t.translated_name` to SELECT:
```diff
- SELECT t.name as tag_name, ...
+ SELECT t.name as tag_name, t.translated_name, ...
```

Frontend `TagTrendItemData` in `App.vue` and `StatsTagTrend.vue` both need `translated_name?: string | null`.

---

## 6. Artwork links → image viewer (Q10)

**Files**: `src-tauri/src/main.rs`, `src-tauri/src/query.rs` (new command), `web/src/App.vue`, `web/src/components/Statistics/StatsTopLists.vue`

### New Tauri command: `get_image_by_id`

```rust
#[tauri::command]
fn get_image_by_id(id: i64) -> Result<query::ImageRow, String> {
    let conn = db::get_conn()?;
    // 1. Query first row (part=0) from images
    //    Use same columns as BASE_SELECT in query.rs
    // 2. Batch fetch all tags for this id (all parts use same tags)
    // 3. Return ImageRow with tags populated
}
```

Multi-page: only need to return `part=0` row with tags — image viewer uses `len`/`part`/`images` to navigate pages. Tags are same across all pages.

### Frontend wiring

**StatsTopLists.vue**: `openArtwork(id)` → calls new handler via emit:
```ts
emit('viewArtwork', id)
```
Remove `window.open(artworkLink(id), '_blank')`.

**App.vue**: handler:
```ts
async function handleViewArtwork(id: number) {
  const row = await invoke<ImageRow>('get_image_by_id', { id })
  const img: Image = {
    id: row.id, part: row.part, len: row.len,
    title: row.title, ext: row.ext,
    size: [row.width, row.height],
    author: { id: row.author_id, name: row.author_name, account: row.author_account },
    tags: row.tags ?? [],
    created_at: row.created_at,
    sanity_level: row.sanity_level,
    x_restrict: row.x_restrict,
    dominant_color: '',
    bookmark: row.bookmark, view: row.view,
    images: { s: row.img_s, m: row.img_m, l: row.img_l, o: row.img_o },
    isAI: row.is_ai,
  }
  store.openImageViewer(img, () => {}, () => {}, -1)
}
```

---

## 7. Empty author name placeholder (Q8)

**Files**: `StatsAuthorChart.vue`, `StatsAuthorDiscovery.vue`, `StatsTopLists.vue`

Replace all `{{ author.author_name }}` with `{{ author.author_name || '(佚名)' }}`.

Affected locations:
- StatsAuthorChart: table cell (line 84), chart bar tooltip label (line 172-173)
- StatsAuthorDiscovery: author name display (line 89)
- StatsTopLists: `item.author_name` in top bookmarked/viewed tables (lines 59, 126, 201)

---

## 8. Horizontal bar chart: largest on top (Q9)

**Files**: `StatsAuthorChart.vue`, `StatsTagChart.vue`

**StatsAuthorChart** (line 169):
```diff
- const reversed = [...top20.value].reverse()
- labels: reversed.map(a => a.author_name)
- data: reversed.map(a => a[sortMode.value])
+ labels: top20.value.map(a => a.author_name)
+ data: top20.value.map(a => a[sortMode.value])
```

**StatsTagChart** (lines 95-96):
```diff
- const labels = top.map(t => t.translated_name || t.name).reverse()
- const counts = top.map(t => t.count).reverse()
+ const labels = top.map(t => t.name)
+ const counts = top.map(t => t.count)
```

---

## 9. Word cloud: d3-cloud (Q11)

**File**: `StatsWordCloud.vue`, `web/package.json`

### Dependencies
```bash
yarn add d3-cloud
yarn add -D @types/d3-cloud
```

### Component rewrite

Replace CSS `flex-wrap` layout with SVG-based d3-cloud rendering:

1. On mount / when tags change → call `cloud()`:
   ```ts
   import cloud from 'd3-cloud'
   
   const words = displayTags.value.map(t => ({
     text: t.name,
     size: fontSize(t.count),   // same sizing logic
     color: tagColor(...),       // same palette
   }))
   
   cloud()
     .size([width, height])
     .words(words)
     .padding(4)
     .rotate(() => (Math.random() > 0.5 ? 0 : 90))
     .on('end', (placed) => { renderedWords.value = placed })
     .start()
   ```

2. Render as SVG `<text>` elements at absolute positions:
   ```html
   <svg :width="w" :height="h">
     <text v-for="w in renderedWords"
       :x="w.x" :y="w.y"
       :font-size="w.size"
       :fill="w.color"
       :transform="`rotate(${w.rotate}, ${w.x}, ${w.y})`"
       text-anchor="middle"
       @click="emit('viewTag', w.text)"
     >{{ w.text }}</text>
   </svg>
   ```

3. Preserve: hover showing `translated_name`, click → emit `viewTag`
4. Sizing: use `ResizeObserver` on container to set SVG dimensions
5. Filter: top 100 tags by count (unchanged)

---

## 10. Author discovery timeline (Q12)

**Files**: `src-tauri/src/stats.rs`, `web/src/components/Statistics/StatsAuthorDiscovery.vue`

### Rust SQL change

```diff
- ORDER BY first_year DESC LIMIT 100
+ HAVING COUNT(DISTINCT i.id) >= 10
+ ORDER BY first_year DESC, works_count DESC
```

- `HAVING works_count >= 10` filters out authors with < 10 bookmarked works
- `ORDER BY first_year DESC` (newest first), then `works_count DESC` (same year, most works first)
- No `LIMIT` — frontend handles per-year cap

### Frontend change

In `groups` computed, after sorting years DESC, apply per-year cap:
```ts
return Array.from(map.entries())
  .map(([year, authors]) => ({
    year,
    authors: authors.slice(0, 100),  // max 100 per year
  }))
  .sort((a, b) => b.year - a.year)
```

---

## Implementation order & dependencies

```
Q9 ─┐
Q8 ─┤
Q7 ─┤  (independent, parallel)
Q2 ─┤
    ├──→ Q4 ─┐ (depends on Q2)
    ├──→ Q6 ─┤ (depends on Q2)
Q5 ─┤  (mostly independent)
Q12┤  (independent)
Q11┤  (independent)
Q10 ┤  (new Tauri command, independent)
```

All items can be decomposed into parallel work units since they touch different files.

---

## Files modified (summary)

| File | Changes |
|------|---------|
| `src-tauri/src/stats.rs` | TagTrendItem +translated_name; AuthorDiscovery HAVING/ORDER |
| `src-tauri/src/main.rs` | New `get_image_by_id` command; register in handler |
| `web/src/App.vue` | `v-show` guard, Q4/Q6/Q10 handlers, proxy emits |
| `web/src/components/Statistics/StatsModal.vue` | `v-show`, full-size, new emits |
| `web/src/components/Statistics/StatsAuthorChart.vue` | Q4 emit, Q8, Q9 (un-reverse) |
| `web/src/components/Statistics/StatsAuthorDiscovery.vue` | Q4 emit, Q8, Q12 slice(0,100) |
| `web/src/components/Statistics/StatsTagChart.vue` | Q5 two-column table, Q6 emit, Q9 (un-reverse) |
| `web/src/components/Statistics/StatsTopLists.vue` | Q8, Q10 emit |
| `web/src/components/Statistics/StatsWordCloud.vue` | Q6 emit, Q11 d3-cloud rewrite |
| `web/src/components/Statistics/StatsTagTrend.vue` | Q5 +translated_name |
| `web/package.json` | +d3-cloud, +@types/d3-cloud |

No new files created (except the spec doc).
