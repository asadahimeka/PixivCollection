<template>
  <div :class="{ dark: colorScheme === 'dark' }">
    <div class="min-h-screen transition-colors dark:bg-[#1a1a1a] dark:text-white">
      <Sidebar />
      <SidebarMask />
      <Navbar @updatebookmark="updateBookmark()" @open-stats="showStatsModal = true; fetchStatistics()" />
      <template v-if="!store.imagesFiltered.length">
        <Tip v-if="loading || store.isFiltering">
          <IconLoading class="mx-auto w-[60px] pb-2" :dark="colorScheme === 'light'" />
          <div class="text-center">
            {{ store.isFiltering ? '筛选中...' : '数据加载中' }}<br>
          </div>
        </Tip>
        <Tip v-if="!loading && !store.isFiltering && !notSettled">
          <div class="text-center">
            暂无数据<br>
          </div>
        </Tip>
        <div v-if="!loading && !store.isFiltering && notSettled" class="my-2 text-center">
          <div class="my-1 p-1">
            设置 Pixiv RefreshToken
            <input
              v-model="pxderToken"
              class="mx-1 w-[330px] rounded-md border px-1 py-0.5 leading-[22px] transition-colors hover:border-blue-500 dark:border-white/40 dark:bg-[#1a1a1a]"
              placeholder="输入 Pixiv RefreshToken 以获取收藏夹数据"
            >
          </div>
          <div class="my-1 p-1">
            设置 HTTP 代理 (TUN 模式无需设置)
            <input
              v-model="pxderProxy"
              class="mx-1 w-[440px] rounded-md border px-1 py-0.5 leading-[22px] transition-colors hover:border-blue-500 dark:border-white/40 dark:bg-[#1a1a1a]"
              placeholder="如果无法直接访问 Pixiv 请设置, 如: http://127.0.0.1:7890"
            >
          </div>
          <div class="my-1 p-1" style="display: flex;justify-content: center;align-items: center;flex-wrap: wrap;">
            设置存储路径
            <CButton class="mx-2" @click="setImgDir">选择图片与数据的存储路径</CButton>
            <p v-if="displayImgDir">{{ displayImgDir }}</p>
          </div>
          <p class="my-2">保存成功后请点击右上角“更新收藏”</p>
          <CButton class="mx-auto my-5 block bg-[#409eff]" @click="saveReload">保存并刷新</CButton>
        </div>
      </template>
      <template v-else>
        <MasonryView />
        <ImageViewer />
        <div ref="sentinel" class="h-px"></div>
        <CButton v-if="!store.loadEnd" class="mx-auto my-5 block" @click="fetchMore">{{ moreLoading ? '加载中' : '加载更多' }}</CButton>
      </template>
      <Transition name="fade">
        <div v-if="showModalMsg" class="bookmark-update-msg">
          <pre ref="modalMsgEl" class="bum-cnt" v-html="modalMsg"></pre>
          <div v-if="reimporting" class="mt-2 text-center text-white">
            <div v-if="reimportStage" class="mb-1">{{ reimportStage }}</div>
            <div>导入进度：{{ reimportProgress.current }} / {{ reimportProgress.total }} 条</div>
          </div>
          <i class="bum-close" @click="closeMsgModal()">×</i>
        </div>
      </Transition>
      <div v-if="showingImport" class="import-progress-overlay">
        <div class="import-progress-box">
          <IconLoading class="mx-auto w-[60px] pb-2" :dark="colorScheme === 'light'" />
          <div class="mt-2 text-center text-lg">{{ importStage }}</div>
          <template v-if="importProgress.total > 0">
            <div class="mt-2 text-center text-sm text-gray-400">
              正在导入 {{ importProgress.current }} / {{ importProgress.total }} 条
            </div>
            <div class="mt-3 h-2 w-64 overflow-hidden rounded-full bg-gray-600">
              <div
                class="h-full rounded-full bg-blue-500 transition-all duration-300"
                :style="{ width: `${importProgress.current / importProgress.total * 100}%` }"
              ></div>
            </div>
          </template>
        </div>
      </div>

      <!-- Statistics Modal -->
      <StatsModal
        :show="showStatsModal"
        :stats="statsData"
        :loading="statsLoading"
        :error="statsError"
        @close="showStatsModal = false"
        @apply-filter="fetchStatistics"
        @retry="fetchStatistics"
      >
        <div v-if="statsData" class="space-y-8">
          <section>
            <StatsOverview
              :overview="statsData.overview"
              :author-ranking="statsData.author_ranking"
              :r18-ratio="statsData.r18_ratio"
              :ai-ratio="statsData.ai_ratio"
            />
          </section>
          <section>
            <StatsAuthorChart :authors="statsData.author_ranking" />
          </section>
          <section>
            <StatsTagChart :tags="statsData.tag_ranking" />
          </section>
          <section>
            <StatsDistribution
              :bookmark-dist="statsData.bookmark_distribution"
              :view-dist="statsData.view_distribution"
            />
          </section>
          <section>
            <StatsTimeChart :yearly="statsData.yearly_trend" />
          </section>
          <section>
            <StatsPieSection
              :r18-ratio="statsData.r18_ratio"
              :ai-ratio="statsData.ai_ratio"
              :shape-dist="statsData.shape_distribution"
              :sanity-levels="statsData.sanity_levels"
            />
          </section>
          <section>
            <StatsTopLists
              :top-bookmarked="statsData.top_bookmarked"
              :top-viewed="statsData.top_viewed"
              :hidden-gems="statsData.hidden_gems"
            />
          </section>
          <section>
            <StatsAuthorDistribution :author-dist="statsData.author_works_distribution" />
          </section>
          <section>
            <StatsWordCloud :tags="statsData.tag_ranking" />
          </section>
          <section>
            <StatsR18AiTrend
              :r18-trend="statsData.r18_trend"
              :ai-trend="statsData.ai_trend"
            />
          </section>
          <section>
            <StatsTagTrend :tag-trend="statsData.tag_trend" />
          </section>
          <section>
            <StatsAuthorDiscovery :author-discovery="statsData.author_discovery" />
          </section>
        </div>
      </StatsModal>
    </div>
  </div>
</template>

<script setup lang="ts">
import { readTextFile, writeTextFile } from '@tauri-apps/api/fs'
import { open as openFileDialog } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import { join } from '@tauri-apps/api/path'
import { Command } from '@tauri-apps/api/shell'

import { SettingType } from '@orilight/vue-settings'
import { useDebounceFn, useIntersectionObserver } from '@vueuse/core'
import { useStore } from '@/store'

import StatsModal from '@/components/Statistics/StatsModal.vue'
import StatsOverview from '@/components/Statistics/StatsOverview.vue'
import StatsAuthorChart from '@/components/Statistics/StatsAuthorChart.vue'
import StatsTagChart from '@/components/Statistics/StatsTagChart.vue'
import StatsTimeChart from '@/components/Statistics/StatsTimeChart.vue'
import StatsDistribution from '@/components/Statistics/StatsDistribution.vue'
import StatsPieSection from '@/components/Statistics/StatsPieSection.vue'
import StatsTopLists from '@/components/Statistics/StatsTopLists.vue'
import StatsAuthorDistribution from '@/components/Statistics/StatsAuthorDistribution.vue'
import StatsWordCloud from '@/components/Statistics/StatsWordCloud.vue'
import StatsR18AiTrend from '@/components/Statistics/StatsR18AiTrend.vue'
import StatsTagTrend from '@/components/Statistics/StatsTagTrend.vue'
import StatsAuthorDiscovery from '@/components/Statistics/StatsAuthorDiscovery.vue'

const store = useStore()

const {
  preferColorScheme,
  colorScheme,
  masonryConfig,
  filterConfig,
} = toRefs(store)

const loading = ref(true)
const moreLoading = ref(false)
const importing = ref(false)
const showingImport = ref(false)
const importProgress = ref({ current: 0, total: 0 })
const importStage = ref('首次导入数据到数据库')
let unlistenImport: (() => void) | null = null
let importShowTimer: ReturnType<typeof setTimeout> | null = null

// ---- Statistics types (matches Rust backend) ----
interface StatsOverviewData {
  total_illustrations: number
  total_authors: number
  total_tags: number
  year_range: [number, number]
}
interface AuthorStatsData {
  author_id: number
  author_name: string
  author_account: string
  illustration_count: number
  total_bookmarks: number
  total_views: number
}
interface TagStatsData {
  name: string
  translated_name: string | null
  count: number
}
interface DistributionBucketData {
  label: string
  min: number
  max: number | null
  count: number
}
interface YearlyStatsData {
  year: number
  count: number
  avg_bookmark: number
  avg_view: number
}
interface R18RatioData { safe: number; r18: number; r18g: number }
interface AiRatioData { ai: number; non_ai: number }
interface ShapeBucketData { shape: string; count: number }
interface TopWorkData { id: number; title: string; value: number; author_name: string }
interface AuthorBucketData { label: string; count: number }
interface R18TrendItemData { year: number; x_restrict: number; count: number }
interface AiTrendItemData { year: number; is_ai: boolean; count: number }
interface TagTrendItemData { year: number; tag_name: string; count: number }
interface HiddenGemData { id: number; title: string; author_name: string; bookmark: number; view: number; ratio: number }
interface AuthorDiscoveryData { author_id: number; author_name: string; author_account: string; first_year: number; works_count: number }
interface SanityLevelBucketData { level: number; count: number }
interface StatisticsResultData {
  overview: StatsOverviewData
  author_ranking: AuthorStatsData[]
  tag_ranking: TagStatsData[]
  bookmark_distribution: DistributionBucketData[]
  view_distribution: DistributionBucketData[]
  yearly_trend: YearlyStatsData[]
  r18_ratio: R18RatioData
  ai_ratio: AiRatioData
  shape_distribution: ShapeBucketData[]
  top_bookmarked: TopWorkData[]
  top_viewed: TopWorkData[]
  author_works_distribution: AuthorBucketData[]
  r18_trend: R18TrendItemData[]
  ai_trend: AiTrendItemData[]
  tag_trend: TagTrendItemData[]
  hidden_gems: HiddenGemData[]
  author_discovery: AuthorDiscoveryData[]
  sanity_levels: SanityLevelBucketData[]
}

const showStatsModal = ref(false)
const statsData = ref<StatisticsResultData | null>(null)
const statsLoading = ref(false)
const statsError = ref<string | null>(null)

async function fetchStatistics(filter?: { year_min?: number | null; year_max?: number | null; r18?: string | null; is_ai?: boolean | null }) {
  statsLoading.value = true
  statsError.value = null
  try {
    const result = await invoke('get_statistics', {
      yearMin: filter?.year_min ?? null,
      yearMax: filter?.year_max ?? null,
      r18: filter?.r18 ?? null,
      isAi: filter?.is_ai ?? null,
    })
    statsData.value = result as unknown as StatisticsResultData
  } catch (e) {
    console.error('Statistics fetch failed:', e)
    statsData.value = null
    statsError.value = String(e)
  } finally {
    statsLoading.value = false
  }
}

const sentinel = ref<HTMLElement>()
useIntersectionObserver(sentinel, ([entry]) => {
  if (entry.isIntersecting && !moreLoading.value && !store.loadEnd) {
    fetchMore()
  }
})

const { __CONFIG__ } = window as any
const notSettled = !(__CONFIG__.imgDir && __CONFIG__.pxderToken)
const pxderToken = ref(__CONFIG__.pxderToken)
const pxderProxy = ref(__CONFIG__.pxderProxy)

const displayImgDir = ref(__CONFIG__.imgDir || '')
async function setImgDir() {
  const imgDir = await openFileDialog({ directory: true })
  if (typeof imgDir == 'string') {
    __CONFIG__.imgDir = imgDir
    displayImgDir.value = imgDir
  }
}

const modalMsgEl = ref<HTMLElement>()
const showModalMsg = ref(false)
const modalMsg = ref('')
const reimporting = ref(false)
const reimportProgress = ref({ current: 0, total: 0 })
const reimportStage = ref('')
watch(modalMsg, () => {
  nextTick(() => {
    modalMsgEl.value?.scrollTo({ top: modalMsgEl.value.scrollHeight })
  })
})

let startUpdateCp: any = null
const killUpdateCp = () => {
  try { startUpdateCp?.kill() } catch (err) {}
}
onUnmounted(killUpdateCp)
async function updateBookmark() {
  showModalMsg.value = true
  modalMsg.value = 'Start updating bookmark...\n'

  const exeDir = await invoke<string>('get_executable_dir')
  console.log('cwd: ', exeDir)
  modalMsg.value += `Current executable dir: ${exeDir}\n`

  const cmdCwd = await join(exeDir, import.meta.env.DEV ? '../../../' : '.', 'pxder')
  const cmdPath = await join(cmdCwd, 'start.bat')
  console.log('cmdPath: ', cmdPath)
  modalMsg.value += `Execute script: ${cmdPath}\n`

  // 创建命令
  const startCmd = new Command('cmd', ['/C', cmdPath], { cwd: cmdCwd })
  startCmd.on('close', async data => {
    const msg = `UpdateBookmark command finished with code ${data.code} and signal ${data.signal}.`
    console.log(msg)
    modalMsg.value += msg

    try {
      modalMsg.value += '\n开始导入数据到数据库...\n'

      const ver = (Number(localStorage.getItem('_images_json_version') || '0')) + 1
      localStorage.setItem('_images_json_version', String(ver))

      reimporting.value = true
      reimportProgress.value = { current: 0, total: 0 }
      reimportStage.value = ''
      const unlistenReimport = await listen<{ current?: number; total?: number; stage?: string }>('import-progress', event => {
        if (event.payload.stage) {
          reimportStage.value = event.payload.stage
        } else if (event.payload.current !== undefined && event.payload.total !== undefined) {
          reimportProgress.value = { current: event.payload.current, total: event.payload.total }
        }
      })

      // reimport_db: Rust side reads images.json from disk (no IPC for large JSON),
      // incrementally imports, then refreshes _meta caches
      const result = await invoke<{ imported: number; skipped: number }>('reimport_db', {
        imgDir: __CONFIG__.imgDir,
        version: ver,
      })

      unlistenReimport()
      reimporting.value = false
      modalMsg.value += `导入完成：${result.imported} 条新数据，${result.skipped} 条跳过\n`

      // Refresh frontend counts from _meta cache
      const counts = await invoke<any>('get_full_counts')
      store.fullCounts.total = counts.total
      store.fullCounts.illustCount = counts.illustCount
      store.fullCounts.authorCount = counts.authorCount
      store.fullCounts.tagCount = counts.tagCount

      // Refresh first page
      await store.loadImagesByPage(true)

      modalMsg.value += '数据库已更新\n'
    } catch (err) {
      reimporting.value = false
      modalMsg.value += `<br><div style="color:#ff6565">导入失败: ${err}</div>`
    }
  })
  startCmd.on('error', error => {
    const msg = `UpdateBookmark command error: "${error}".`
    console.error(msg)
    modalMsg.value += `<br><div style="color:#ff6565">${msg}</div>`
  })
  // 监听 stdout 实时输出
  startCmd.stdout.on('data', (line: string) => {
    line = line.replaceAll('\x1B[2K\x1B[1G', '')
    console.log('STDOUT: ', line)
    // 在这里处理实时输出，例如更新UI
    const str = line.split('__EOS__').filter(Boolean).map(e => {
      const m = e.match(/(.*)\$\${%c\<(.*)\>%(.*)}\$\$(.*)/)
      return m ? `<span>${m[1] || ''}<span style="${m[2] || ''}">${m[3] || ''}</span>${m[4] || ''}</span>` : e
    }).join('')
    modalMsg.value += str ? `<div>${str}</div>` : (line || '<br>')
  })
  // 监听 stderr 实时输出
  startCmd.stderr.on('data', (line: string) => {
    console.error('STDERR: ', line)
    // 在这里处理错误输出
    const str = line.split('__EOS__').filter(Boolean).map(e => {
      const m = e.match(/(.*)\$\${(%c)?\<(.*)\>%(.*)}\$\$(.*)/)
      return m ? `<span>${m[1] || ''}<span style="${m[3]?.replaceAll('red', '#ff6565') || ''}">${m[4] || ''}</span>${m[5] || ''}</span>` : e
    }).join('')
    modalMsg.value += str ? `<div style="color:#ff6565">${str}</div>` : (line ? `<div style="color:#ff6565">${line}</span>` : '<br>')
  })
  // 执行命令
  startUpdateCp = await startCmd.spawn()
}

function closeMsgModal() {
  showModalMsg.value = false
  modalMsg.value = ''
  killUpdateCp()
  // No more location.reload() — data is already updated via reimport_db
}

async function saveReload() {
  if (!/^[\w-]{43}$/.test(pxderToken.value)) {
    alert('请输入有效的 RefreshToken')
    return
  }
  if (pxderProxy.value && !/^http\:\/\/\d+\.\d+\.\d+\.\d+\:\d+$/.test(pxderProxy.value)) {
    alert('请输入有效的 HTTP 代理')
    return
  }
  if (!__CONFIG__.imgDir?.trim()) {
    alert('请设置存储路径')
    return
  }

  localStorage.setItem('__PXCT_IMG_DIR', __CONFIG__.imgDir)
  localStorage.setItem('__PXCT_PXDER_TOKEN', pxderToken.value)
  localStorage.setItem('__PXCT_PXDER_PROXY', pxderProxy.value)

  const cwd = await invoke<string>('get_executable_dir')
  const pxderPath = await join(cwd, import.meta.env.DEV ? '../../../' : '.', 'pxder')
  const configPath = await join(pxderPath, 'src/config/config.json')
  const configJson = JSON.parse(await readTextFile(configPath).catch(() => '{"download":{}}'))
  configJson.download.path = __CONFIG__.imgDir
  configJson.refresh_token = pxderToken.value
  configJson.proxy = pxderProxy.value
  await writeTextFile(configPath, JSON.stringify(configJson))
  await writeTextFile(await join(pxderPath, 'data/_last_id.json'), '{"id":""}').catch(() => {})
  await writeTextFile(await join(pxderPath, 'data/_last_illusts.json'), '[]').catch(() => {})
  await invoke('restart_app')
}

onMounted(async () => {
  store.settings.register('preferColorScheme', preferColorScheme as any)
  store.settings.register('masonryConfig', masonryConfig as any, SettingType.Json, {
    deepMerge: true,
  })
  store.settings.register('restrictConfig', toRef(filterConfig.value, 'restrict') as any, SettingType.Json, {
    deepMerge: true,
  })

  await init()
})

onUnmounted(() => {
  store.settings.unregisterAll()
})

const isInit = ref(false)
watch(
  () => store.filterConfig,
  useDebounceFn(() => {
    if (!isInit.value) return
    store.curPageCursor = 0
    store.loadEnd = false
    store.imagesFiltered = []
    store.loadImagesByPage(true)
    document.documentElement.scrollTop = 0
  }, 250),
  { deep: true },
)

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
      const verStr = localStorage.getItem('_images_json_version')

      importing.value = true
      showingImport.value = false
      importProgress.value = { current: 0, total: 0 }
      importShowTimer = setTimeout(() => {
        if (importing.value) showingImport.value = true
      }, 1000)
      unlistenImport = await listen<{ current?: number; total?: number; stage?: string }>('import-progress', event => {
        if (event.payload.stage) {
          importStage.value = event.payload.stage
        } else if (event.payload.current !== undefined && event.payload.total !== undefined) {
          importProgress.value = { current: event.payload.current, total: event.payload.total }
        }
      })

      const ensureResult = await invoke<{ status: string }>('ensure_db', {
        imgDir: __CONFIG__.imgDir,
        version: verStr ? Number(verStr) : null,
      })

      if (importShowTimer) { clearTimeout(importShowTimer); importShowTimer = null }
      unlistenImport?.()
      unlistenImport = null
      showingImport.value = false
      importing.value = false

      // Populate full (unfiltered) counts for the sidebar "总计" row
      // Read cached full counts from _meta (instant, no aggregate SQL)
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
      await store.loadImagesByPage(true)
    }
  } catch (e) {
    console.error(e)
    const msg = (e as Error).message || JSON.stringify(e)
    showModalMsg.value = true
    modalMsg.value += `<br><div style="color:#ff6565">${msg}</div>`
  } finally {
    loading.value = false
    importing.value = false
    showingImport.value = false
    if (importShowTimer) { clearTimeout(importShowTimer); importShowTimer = null }
    if (unlistenImport) {
      unlistenImport()
      unlistenImport = null
    }
    setTimeout(() => {
      isInit.value = true
    }, 500)
  }
}

async function fetchMore() {
  if (moreLoading.value) return
  moreLoading.value = true
  if (__CONFIG__.imgDir) {
    await store.loadImagesByPage()
  } else {
    store.loadEnd = true
  }
  moreLoading.value = false
}
</script>

<style>
body {
  overflow-y: scroll;
}

body:has(.bookmark-update-msg) {
  overflow-y: hidden;
}

.bookmark-update-msg {
  position: fixed;
  z-index: 200;
  top: 0;
  left: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.9);
  overflow-y: auto;
}

.bookmark-update-msg .bum-cnt {
  min-width: 500px;
  max-width: 98vw;
  max-height: 100vh;
  padding: 20px;
  color: white;
  white-space: pre-wrap;
  font-family: Consolas, 'Courier New', Courier, monospace;
  font-size: 18px;
  overflow-y: auto;
}

.bookmark-update-msg .bum-close {
  position: absolute;
  top: 16px;
  right: 36px;
  font-size: 36px;
  padding: 10px;
  color: white;
  font-style: normal;
  font-family: SimSun, monospace;
  font-weight: bold;
  cursor: pointer;
}

.import-progress-overlay {
  position: fixed;
  z-index: 200;
  top: 0;
  left: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.85);
}

.import-progress-box {
  text-align: center;
  color: white;
}
</style>
