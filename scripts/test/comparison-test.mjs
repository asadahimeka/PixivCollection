#!/usr/bin/env node
/**
 * scripts/comparison-test.mjs
 *
 * Compares JS filter logic (from the old store/index.ts) vs SQL query builder
 * (from src-tauri/src/query.rs) on the same dataset to verify parity.
 *
 * Reads images.json (JS objects) and images.db (SQLite), runs each filter
 * combination through both paths, and reports PASS/FAIL.
 *
 * Usage:
 *   node scripts/comparison-test.mjs
 *   IMG_DIR=/path/to/images node scripts/comparison-test.mjs
 *
 * Requires: npm install better-sqlite3
 *
 * Exit code: 0 if all pass, 1 if any mismatch
 */

import { readFileSync, existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import Database from 'better-sqlite3'

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------
const IMG_DIR = resolve(process.env.IMG_DIR || '.')
const JSON_PATH = join(IMG_DIR, 'data', 'images.json')
const DB_PATH = join(IMG_DIR, 'data', 'images.db')

// ---------------------------------------------------------------------------
// JS Filter Logic (replicated from old store/index.ts imageFilter + getSearchStr)
// ---------------------------------------------------------------------------

function getSearchStr(image) {
  return (
    String(image.id)
    + image.title
    + String(image.author.id)
    + image.author.name
    + image.tags
      .map(tag => tag.translated_name
        ? tag.name + tag.translated_name
        : tag.name,
      ).join()
  ).toLowerCase()
}

function imageFilter(image, filterConfig) {
  // R18
  if (filterConfig.restrict.r18 === 'hidden') {
    if (image.x_restrict >= 1) return false
  }
  else if (filterConfig.restrict.r18 === 'only') {
    if (image.x_restrict < 1) return false
  }

  // Max sanity level
  if (filterConfig.restrict.maxSanityLevel) {
    if (image.sanity_level > filterConfig.restrict.maxSanityLevel) return false
  }

  // Search
  if (filterConfig.search.enable) {
    const term = filterConfig.search.value.trim().toLowerCase()
    if (term !== '' && !getSearchStr(image).includes(term)) return false
  }

  // Bookmark
  if (filterConfig.bookmark.enable) {
    if (filterConfig.bookmark.min === -1) {
      if (image.bookmark !== -1) return false
    }
    if (image.bookmark < filterConfig.bookmark.min) return false
  }

  // Year
  if (filterConfig.year.enable) {
    const year = Number(image.created_at.split('-')[0])
    if (filterConfig.year.value === 1) {
      if (year > 2000) return false
    }
    else if (year !== filterConfig.year.value) {
      return false
    }
  }

  // Author
  if (filterConfig.author.enable) {
    if (image.author.id !== filterConfig.author.id) return false
  }

  // Tag (exact match)
  if (filterConfig.tag.enable) {
    if (!image.tags.find(tag => tag.name === filterConfig.tag.name)) return false
  }

  // Shape
  if (filterConfig.shape.enable) {
    const value = filterConfig.shape.value
    const ratio = image.size[0] / image.size[1]
    if (value.startsWith('ratio-')) {
      const parts = value.substring(6).split(':').map(Number)
      const targetRatio = parts[0] / parts[1]
      if (ratio >= targetRatio / 0.9 || ratio <= targetRatio * 0.9) return false
    }
    else if (ratio < 0.9 || ratio > 1.1) {
      if (value === 'square') return false
      if (image.size[0] > image.size[1]) {
        if (value === 'vertical') return false
      }
      else {
        if (value === 'horizontal') return false
      }
    }
    else {
      if (value === 'vertical' || value === 'horizontal') return false
    }
  }

  // Size
  if (filterConfig.size.enable) {
    if (filterConfig.size.width.max && image.size[0] > filterConfig.size.width.max) return false
    if (filterConfig.size.width.min && image.size[0] < filterConfig.size.width.min) return false
    if (filterConfig.size.height.max && image.size[1] > filterConfig.size.height.max) return false
    if (filterConfig.size.height.min && image.size[1] < filterConfig.size.height.min) return false
  }

  return true
}

// ---------------------------------------------------------------------------
// SQL Query Builder (matching query.rs build_where() exactly)
// ---------------------------------------------------------------------------

function buildSQLWhere(filterConfig) {
  const conditions = []
  const params = []

  // R18
  if (filterConfig.restrict.r18 === 'hidden')
    conditions.push('i.x_restrict < 1')
  else if (filterConfig.restrict.r18 === 'only')
    conditions.push('i.x_restrict >= 1')

  // Max sanity level
  if (filterConfig.restrict.maxSanityLevel) {
    conditions.push('i.sanity_level <= ?')
    params.push(filterConfig.restrict.maxSanityLevel)
  }

  // Search
  if (filterConfig.search.enable && filterConfig.search.value.trim()) {
    const term = `%${filterConfig.search.value.trim().toLowerCase()}%`
    conditions.push(
      '(CAST(i.id AS TEXT) LIKE ? OR i.title LIKE ? OR CAST(i.author_id AS TEXT) LIKE ? OR i.author_name LIKE ? OR EXISTS(SELECT 1 FROM image_tags it JOIN tags t ON it.tag_id = t.id WHERE it.image_id = i.id AND it.image_part = i.part AND (LOWER(t.name) LIKE LOWER(?) OR LOWER(t.translated_name) LIKE LOWER(?))))',
    )
    for (let j = 0; j < 6; j++)
      params.push(term)
  }

  // Bookmark
  if (filterConfig.bookmark.enable) {
    if (filterConfig.bookmark.min === -1) {
      conditions.push('i.bookmark = -1')
    }
    else if (filterConfig.bookmark.min >= 0) {
      conditions.push('i.bookmark >= ?')
      params.push(filterConfig.bookmark.min)
    }
  }

  // Year
  if (filterConfig.year.enable) {
    if (filterConfig.year.value === 1) {
      conditions.push('CAST(substr(i.created_at,1,4) AS INTEGER) < 2000')
    }
    else {
      conditions.push('CAST(substr(i.created_at,1,4) AS INTEGER) = ?')
      params.push(filterConfig.year.value)
    }
  }

  // Tag
  if (filterConfig.tag.enable && filterConfig.tag.name) {
    conditions.push(
      'EXISTS(SELECT 1 FROM image_tags it JOIN tags t ON it.tag_id = t.id WHERE it.image_id = i.id AND it.image_part = i.part AND t.name = ?)',
    )
    params.push(filterConfig.tag.name)
  }

  // Author
  if (filterConfig.author.enable && filterConfig.author.id !== -1) {
    conditions.push('i.author_id = ?')
    params.push(filterConfig.author.id)
  }

  // Shape
  if (filterConfig.shape.enable && filterConfig.shape.value) {
    const s = filterConfig.shape.value
    if (s === 'horizontal')
      conditions.push('i.width > i.height * 1.1')
    else if (s === 'vertical')
      conditions.push('i.width < i.height * 0.9')
    else if (s === 'square')
      conditions.push('i.width BETWEEN i.height * 0.9 AND i.height * 1.1')
    else if (s === 'ratio-4:3')
      conditions.push('ABS(i.width * 3 - i.height * 4) * 100 <= (i.width * 3 + i.height * 4) / 2')
    else if (s === 'ratio-16:9')
      conditions.push('ABS(i.width * 9 - i.height * 16) * 100 <= (i.width * 9 + i.height * 16) / 2')
    else if (s === 'ratio-21:9')
      conditions.push('ABS(i.width * 9 - i.height * 21) * 100 <= (i.width * 9 + i.height * 21) / 2')
    else if (s === 'ratio-3:4')
      conditions.push('ABS(i.width * 4 - i.height * 3) * 100 <= (i.width * 4 + i.height * 3) / 2')
    else if (s === 'ratio-9:16')
      conditions.push('ABS(i.width * 16 - i.height * 9) * 100 <= (i.width * 16 + i.height * 9) / 2')
    else if (s === 'ratio-9:21')
      conditions.push('ABS(i.width * 21 - i.height * 9) * 100 <= (i.width * 21 + i.height * 9) / 2')
  }

  // Size
  if (filterConfig.size.enable) {
    const w = filterConfig.size.width
    const h = filterConfig.size.height
    if (w.min !== null) {
      conditions.push('i.width >= ?')
      params.push(w.min)
    }
    if (w.max !== null) {
      conditions.push('i.width <= ?')
      params.push(w.max)
    }
    if (h.min !== null) {
      conditions.push('i.height >= ?')
      params.push(h.min)
    }
    if (h.max !== null) {
      conditions.push('i.height <= ?')
      params.push(h.max)
    }
  }

  return { conditions, params }
}

// ---------------------------------------------------------------------------
// ID helpers
// ---------------------------------------------------------------------------

function imageId(image) {
  return `${image.id}_${image.part}`
}

function cmpSets(jsSet, sqlSet) {
  const onlyJS = [...jsSet].filter(k => !sqlSet.has(k))
  const onlySQL = [...sqlSet].filter(k => !jsSet.has(k))
  return { onlyJS, onlySQL, match: onlyJS.length === 0 && onlySQL.length === 0 }
}

// ---------------------------------------------------------------------------
// Data loading
// ---------------------------------------------------------------------------

function loadJSONImages(jsonPath) {
  if (!existsSync(jsonPath)) {
    console.error(`ERROR: images.json not found at ${jsonPath}`)
    process.exit(1)
  }
  const raw = readFileSync(jsonPath, 'utf-8')
  const data = JSON.parse(raw)
  console.log(`Loaded ${data.length} images from ${jsonPath}`)
  return data
}

function openDB(dbPath) {
  if (!existsSync(dbPath)) {
    console.error(`ERROR: images.db not found at ${dbPath}`)
    process.exit(1)
  }
  const db = new Database(dbPath)
  console.log(`Opened SQLite DB at ${dbPath}\n`)
  return db
}

function getSQLCount(db, conditions, params) {
  let sql = 'SELECT COUNT(*) as total FROM images i WHERE 1=1'
  if (conditions.length > 0)
    sql += ' AND ' + conditions.join(' AND ')

  const row = db.prepare(sql).get(...params)
  return row.total
}

function getSQLIds(db, conditions, params) {
  let sql = "SELECT i.id || '_' || i.part as id_part FROM images i WHERE 1=1"
  if (conditions.length > 0)
    sql += ' AND ' + conditions.join(' AND ')

  const rows = db.prepare(sql).all(...params)
  return new Set(rows.map(r => r.id_part))
}

// ---------------------------------------------------------------------------
// Default filter config (matches store default)
// ---------------------------------------------------------------------------

function defaultConfig() {
  return {
    search: { enable: false, value: '' },
    year: { enable: false, value: 0 },
    tag: { enable: false, name: '', includeBookmark: false, includeRatherThan: 5 },
    author: { enable: false, id: -1 },
    shape: { enable: false, value: '' },
    size: { enable: false, width: { max: null, min: null }, height: { max: null, min: null } },
    bookmark: { enable: false, min: 0 },
    restrict: { maxSanityLevel: null, r18: 'show' },
  }
}

// ---------------------------------------------------------------------------
// Test infrastructure
// ---------------------------------------------------------------------------

let passed = 0
let failed = 0
const failures = []

function runTest(name, images, db, makeConfig) {
  const config = makeConfig(defaultConfig())

  // --- JS filter ---
  const jsIds = new Set(
    images
      .filter(img => imageFilter(img, config))
      .map(img => imageId(img)),
  )

  // --- SQL filter ---
  const { conditions, params } = buildSQLWhere(config)
  const sqlIds = getSQLIds(db, conditions, params)

  // --- Compare ---
  const { onlyJS, onlySQL, match } = cmpSets(jsIds, sqlIds)

  process.stdout.write(`Test: ${name}\n`)
  process.stdout.write(`  JS count: ${jsIds.size}  SQL count: ${sqlIds.size}`)

  if (jsIds.size === sqlIds.size) {
    process.stdout.write('  Count: ✓\n')
  }
  else {
    process.stdout.write(`  Count: ✗ (diff: ${jsIds.size - sqlIds.size})\n`)
  }

  if (match) {
    process.stdout.write('  IDs match: ✓\n')
    process.stdout.write('  Result: PASS\n\n')
    passed++
  }
  else {
    process.stdout.write(`  IDs match: ✗\n`)
    if (onlyJS.length > 0)
      process.stdout.write(`    Only in JS (first 10): ${onlyJS.slice(0, 10).join(', ')}\n`)
    if (onlySQL.length > 0)
      process.stdout.write(`    Only in SQL (first 10): ${onlySQL.slice(0, 10).join(', ')}\n`)
    process.stdout.write('  Result: FAIL\n\n')
    failed++
    failures.push(name)
  }
}

// ---------------------------------------------------------------------------
// Data sampling helpers (to build data-driven test configs)
// ---------------------------------------------------------------------------

function firstAuthor(images) {
  for (const img of images) {
    if (img.author && img.author.id)
      return { id: img.author.id, name: img.author.name }
  }
  return null
}

function firstTag(images) {
  const seen = new Set()
  for (const img of images) {
    for (const tag of img.tags) {
      if (tag.name && !seen.has(tag.name)) {
        seen.add(tag.name)
        return tag
      }
    }
  }
  return null
}

function firstYear(images) {
  const years = new Set()
  for (const img of images) {
    const y = Number(img.created_at.split('-')[0])
    if (y > 2000)
      years.add(y)
  }
  const sorted = [...years].sort((a, b) => a - b)
  return sorted.length > 0 ? sorted[0] : null
}

function anyTitleWord(images) {
  for (const img of images) {
    const words = img.title.trim().split(/\s+/)
    for (const w of words) {
      if (w.length >= 3)
        return w
    }
  }
  return null
}

function anySearchable(images, kind) {
  // kind: 'tag_name', 'author_name', 'id_str'
  if (kind === 'tag_name') {
    const tag = firstTag(images)
    return tag ? tag.name : null
  }
  if (kind === 'author_name') {
    const a = firstAuthor(images)
    return a ? a.name : null
  }
  if (kind === 'id_str') {
    return images.length > 0 ? String(images[0].id) : null
  }
  return null
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main() {
  const images = loadJSONImages(JSON_PATH)
  const db = openDB(DB_PATH)

  // Derive data-driven filter values
  const author = firstAuthor(images)
  const tag = firstTag(images)
  const yearVal = firstYear(images)
  const titleWord = anyTitleWord(images)
  const searchTagName = anySearchable(images, 'tag_name')
  const searchAuthorName = anySearchable(images, 'author_name')
  const searchIdStr = anySearchable(images, 'id_str')

  // ---- Define test cases ----

  const tests = []

  // 1. No filters (all images)
  tests.push({
    name: 'No filters (all images)',
    fn: () => defaultConfig(),
  })

  // 2. R18 hidden
  tests.push({
    name: 'R18 hidden',
    fn: cfg => { cfg.restrict.r18 = 'hidden'; return cfg },
  })

  // 3. R18 only
  tests.push({
    name: 'R18 only',
    fn: cfg => { cfg.restrict.r18 = 'only'; return cfg },
  })

  // 4. Search by tag name
  if (searchTagName) {
    tests.push({
      name: `Search by tag name ("${searchTagName}")`,
      fn: cfg => {
        cfg.search.enable = true
        cfg.search.value = searchTagName
        return cfg
      },
    })
  }

  // 5. Search by title word
  if (titleWord) {
    tests.push({
      name: `Search by title word ("${titleWord}")`,
      fn: cfg => {
        cfg.search.enable = true
        cfg.search.value = titleWord
        return cfg
      },
    })
  }

  // 6. Search by author name
  if (searchAuthorName) {
    tests.push({
      name: `Search by author name ("${searchAuthorName}")`,
      fn: cfg => {
        cfg.search.enable = true
        cfg.search.value = searchAuthorName
        return cfg
      },
    })
  }

  // 7. Search by illust ID (as string)
  if (searchIdStr) {
    tests.push({
      name: `Search by illust ID ("${searchIdStr}")`,
      fn: cfg => {
        cfg.search.enable = true
        cfg.search.value = searchIdStr
        return cfg
      },
    })
  }

  // 8. Author filter
  if (author) {
    tests.push({
      name: `Author filter (id=${author.id} "${author.name}")`,
      fn: cfg => {
        cfg.author.enable = true
        cfg.author.id = author.id
        return cfg
      },
    })
  }

  // 9. Tag filter
  if (tag) {
    tests.push({
      name: `Tag filter ("${tag.name}")`,
      fn: cfg => {
        cfg.tag.enable = true
        cfg.tag.name = tag.name
        return cfg
      },
    })
  }

  // 10. Year filter (specific year)
  if (yearVal) {
    tests.push({
      name: `Year filter (${yearVal})`,
      fn: cfg => {
        cfg.year.enable = true
        cfg.year.value = yearVal
        return cfg
      },
    })
  }

  // 11. Year filter (before 2000)
  tests.push({
    name: 'Year filter (before 2000, value=1)',
    fn: cfg => {
      cfg.year.enable = true
      cfg.year.value = 1
      return cfg
    },
  })

  // 12. Bookmark min (use a threshold that some images meet)
  {
    // Find a reasonable bookmark threshold from the data
    const bmValues = images.map(i => i.bookmark).filter(b => b > 0).sort((a, b) => a - b)
    const bmThreshold = bmValues.length > 10 ? bmValues[Math.floor(bmValues.length * 0.3)] : 100
    tests.push({
      name: `Bookmark min (>= ${bmThreshold})`,
      fn: cfg => {
        cfg.bookmark.enable = true
        cfg.bookmark.min = bmThreshold
        return cfg
      },
    })
  }

  // 13. Bookmark = -1 (unbookmarked)
  tests.push({
    name: 'Bookmark = -1 (unbookmarked)',
    fn: cfg => {
      cfg.bookmark.enable = true
      cfg.bookmark.min = -1
      return cfg
    },
  })

  // 14. Shape = square
  tests.push({
    name: 'Shape = square',
    fn: cfg => {
      cfg.shape.enable = true
      cfg.shape.value = 'square'
      return cfg
    },
  })

  // 15. Shape = horizontal
  tests.push({
    name: 'Shape = horizontal',
    fn: cfg => {
      cfg.shape.enable = true
      cfg.shape.value = 'horizontal'
      return cfg
    },
  })

  // 16. Shape = ratio-4:3
  tests.push({
    name: 'Shape = ratio-4:3',
    fn: cfg => {
      cfg.shape.enable = true
      cfg.shape.value = 'ratio-4:3'
      return cfg
    },
  })

  // 17. Size min/max width
  tests.push({
    name: 'Size width (min=400, max=1200)',
    fn: cfg => {
      cfg.size.enable = true
      cfg.size.width.min = 400
      cfg.size.width.max = 1200
      return cfg
    },
  })

  // 18. Size min/max height
  tests.push({
    name: 'Size height (min=400, max=1200)',
    fn: cfg => {
      cfg.size.enable = true
      cfg.size.height.min = 400
      cfg.size.height.max = 1200
      return cfg
    },
  })

  // 19. Combined: year + author + bookmark
  if (author && yearVal) {
    tests.push({
      name: `Combined: year(${yearVal}) + author(${author.id}) + bookmark(>=100)`,
      fn: cfg => {
        cfg.year.enable = true
        cfg.year.value = yearVal
        cfg.author.enable = true
        cfg.author.id = author.id
        cfg.bookmark.enable = true
        cfg.bookmark.min = 100
        return cfg
      },
    })
  }

  // 20. Combined: search + R18 + shape
  if (searchTagName) {
    tests.push({
      name: `Combined: search("${searchTagName}") + R18(hidden) + shape(square)`,
      fn: cfg => {
        cfg.search.enable = true
        cfg.search.value = searchTagName
        cfg.restrict.r18 = 'hidden'
        cfg.shape.enable = true
        cfg.shape.value = 'square'
        return cfg
      },
    })
  }

  // ---- Execute tests ----
  console.log(`Running ${tests.length} comparison tests...\n`)

  for (const test of tests) {
    runTest(test.name, images, db, test.fn)
  }

  // ---- Summary ----
  const total = passed + failed
  console.log('='.repeat(50))
  console.log(`Results: ${passed}/${total} passed`)
  if (failures.length > 0) {
    console.log(`Failed: ${failures.join(', ')}`)
  }

  db.close()
  process.exit(failed > 0 ? 1 : 0)
}

main()
