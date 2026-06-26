<template>
  <Transition name="fade">
    <div
      v-if="show"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      @click.self="emit('close')"
    >
      <div
        class="relative flex max-h-[90vh] w-full max-w-[90vw] flex-col rounded-2xl bg-white shadow-2xl dark:bg-[#242424] dark:text-white"
      >
        <!-- Header -->
        <div class="flex items-center justify-between border-b px-6 py-4 dark:border-white/20">
          <h2 class="text-lg font-bold">
            统计分析
          </h2>
          <button
            class="flex h-8 w-8 items-center justify-center rounded-full transition-colors hover:bg-gray-200 dark:hover:bg-white/10"
            @click="emit('close')"
          >
            <IconClose class="h-5 w-5" />
          </button>
        </div>

        <!-- Filter bar -->
        <div class="border-b px-6 py-3 dark:border-white/20">
          <div class="flex flex-wrap items-center gap-3">
            <!-- Year range -->
            <div class="flex items-center gap-1">
              <span class="text-sm text-gray-600 dark:text-gray-400">年份</span>
              <select
                v-model.number="localFilter.year_min"
                class="rounded-md border px-2 py-1 text-sm transition-colors hover:border-blue-500 dark:border-white/20 dark:bg-[#1a1a1a] dark:hover:border-blue-500"
              >
                <option :value="-1">不限</option>
                <option v-for="y in yearOptions" :key="y" :value="y">{{ y }}</option>
              </select>
              <span class="text-gray-400">-</span>
              <select
                v-model.number="localFilter.year_max"
                class="rounded-md border px-2 py-1 text-sm transition-colors hover:border-blue-500 dark:border-white/20 dark:bg-[#1a1a1a] dark:hover:border-blue-500"
              >
                <option :value="-1">不限</option>
                <option v-for="y in yearOptions" :key="y" :value="y">{{ y }}</option>
              </select>
            </div>

            <!-- R18 tri-state -->
            <div class="flex items-center gap-1">
              <span class="text-sm text-gray-600 dark:text-gray-400">R18</span>
              <div class="flex overflow-hidden rounded-md border dark:border-white/20">
                <button
                  v-for="opt in r18Options" :key="opt.value ?? '__all__'"
                  class="px-2 py-1 text-sm transition-colors"
                  :class="localFilter.r18 === opt.value
                    ? 'bg-blue-500 text-white'
                    : 'hover:bg-gray-100 dark:hover:bg-white/10'"
                  @click="localFilter.r18 = opt.value"
                >
                  {{ opt.label }}
                </button>
              </div>
            </div>

            <!-- AI tri-state -->
            <div class="flex items-center gap-1">
              <span class="text-sm text-gray-600 dark:text-gray-400">AI</span>
              <div class="flex overflow-hidden rounded-md border dark:border-white/20">
                <button
                  v-for="opt in aiOptions" :key="String(opt.value)"
                  class="px-2 py-1 text-sm transition-colors"
                  :class="localFilter.is_ai === opt.value
                    ? 'bg-blue-500 text-white'
                    : 'hover:bg-gray-100 dark:hover:bg-white/10'"
                  @click="localFilter.is_ai = opt.value"
                >
                  {{ opt.label }}
                </button>
              </div>
            </div>

            <!-- Apply button -->
            <button
              class="rounded-md bg-blue-500 px-4 py-1 text-sm text-white transition-colors hover:bg-blue-600"
              @click="applyFilter"
            >
              应用
            </button>
          </div>

          <!-- Active filter chips -->
          <div v-if="hasActiveFilters" class="mt-2 flex flex-wrap gap-1">
            <span
              v-if="localFilter.year_min !== -1 || localFilter.year_max !== -1"
              class="inline-flex items-center gap-1 rounded-sm bg-yellow-300/20 px-1.5 py-0.5 text-xs"
            >
              年份: {{ localFilter.year_min === -1 ? '不限' : localFilter.year_min }}
              ~ {{ localFilter.year_max === -1 ? '不限' : localFilter.year_max }}
              <button class="ml-0.5 leading-none hover:text-red-500" @click="clearYear">&times;</button>
            </span>
            <span
              v-if="localFilter.r18 !== null"
              class="inline-flex items-center gap-1 rounded-sm bg-red-500/20 px-1.5 py-0.5 text-xs"
            >
              R18: {{ r18Label }}
              <button class="ml-0.5 leading-none hover:text-red-500" @click="localFilter.r18 = null">&times;</button>
            </span>
            <span
              v-if="localFilter.is_ai !== null"
              class="inline-flex items-center gap-1 rounded-sm bg-blue-500/20 px-1.5 py-0.5 text-xs"
            >
              AI: {{ aiLabel }}
              <button class="ml-0.5 leading-none hover:text-red-500" @click="localFilter.is_ai = null">&times;</button>
            </span>
          </div>
        </div>

        <!-- Content area -->
        <div class="flex-1 overflow-y-auto p-6">
          <!-- Loading skeleton -->
          <div v-if="loading" class="animate-pulse space-y-6 p-6">
            <div class="h-24 rounded-xl bg-gray-200 dark:bg-white/10"></div>
            <div class="h-64 rounded-xl bg-gray-200 dark:bg-white/10"></div>
            <div class="h-48 rounded-xl bg-gray-200 dark:bg-white/10"></div>
          </div>

          <!-- Error state -->
          <div v-else-if="error" class="flex flex-col items-center justify-center py-20 text-gray-400">
            <p class="mb-4">统计数据加载失败</p>
            <p class="mb-4 text-sm text-red-400">{{ error }}</p>
            <button
              class="rounded-md bg-blue-500 px-4 py-2 text-sm text-white hover:bg-blue-600"
              @click="emit('retry')"
            >
              重试
            </button>
          </div>

          <!-- Empty state -->
          <div v-else-if="!stats" class="flex items-center justify-center py-20 text-gray-400">
            <p>暂无统计数据</p>
          </div>

          <!-- Loaded content -->
          <slot v-else></slot>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { useStore } from '@/store'

// ---- Props & Emits ----

const props = defineProps<{
  show: boolean
  stats: StatisticsResult | null
  loading: boolean
  error: string | null
}>()

const emit = defineEmits<{
  close: []
  'applyFilter': [filter: StatsFilter]
  retry: []
}>()

const store = useStore()

// ---- Types (matches Rust backend serde serialization — snake_case) ----

interface StatsOverview {
  total_illustrations: number // i64
  total_authors: number // i64
  total_tags: number // i64
  year_range: [number, number] // (i32, i32) -> JSON array
}

interface AuthorStats {
  author_id: number // i64
  author_name: string // String
  author_account: string // String
  illustration_count: number // i64
  total_bookmarks: number // i64
  total_views: number // i64
}

interface TagStats {
  name: string // String
  translated_name: string | null // Option<String>
  count: number // i64
}

interface DistributionBucket {
  label: string // String
  min: number // i64
  max: number | null // Option<i64>
  count: number // i64
}

interface YearlyStats {
  year: number // i32
  count: number // i64
  avg_bookmark: number // f64
  avg_view: number // f64
}

interface R18Ratio {
  safe: number // i64
  r18: number // i64
  r18g: number // i64
}

interface AiRatio {
  ai: number // i64
  non_ai: number // i64
}

interface ShapeBucket {
  shape: string // String
  count: number // i64
}

interface TopWork {
  id: number // i64
  title: string // String
  value: number // i64
  author_name: string // String
}

interface AuthorBucket {
  label: string // String
  count: number // i64
}

interface R18TrendItem {
  year: number // i32
  x_restrict: number // i32
  count: number // i64
}

interface AiTrendItem {
  year: number // i32
  is_ai: boolean // bool
  count: number // i64
}

interface TagTrendItem {
  year: number // i32
  tag_name: string // String
  count: number // i64
}

interface HiddenGem {
  id: number // i64
  title: string // String
  author_name: string // String
  bookmark: number // i64
  view: number // i64
  ratio: number // f64
}

interface AuthorDiscovery {
  author_id: number // i64
  author_name: string // String
  author_account: string // String
  first_year: number // i32
  works_count: number // i64
}

interface SanityLevelBucket {
  level: number // i32
  count: number // i64
}

interface StatisticsResult {
  overview: StatsOverview
  author_ranking: AuthorStats[]
  tag_ranking: TagStats[]
  bookmark_distribution: DistributionBucket[]
  view_distribution: DistributionBucket[]
  yearly_trend: YearlyStats[]
  r18_ratio: R18Ratio
  ai_ratio: AiRatio
  shape_distribution: ShapeBucket[]
  top_bookmarked: TopWork[]
  top_viewed: TopWork[]
  author_works_distribution: AuthorBucket[]
  r18_trend: R18TrendItem[]
  ai_trend: AiTrendItem[]
  tag_trend: TagTrendItem[]
  hidden_gems: HiddenGem[]
  author_discovery: AuthorDiscovery[]
  sanity_levels: SanityLevelBucket[]
}

interface StatsFilter {
  year_min: number | null
  year_max: number | null
  r18: string | null // "show" | "hidden" | "only" | null
  is_ai: boolean | null
}

// ---- Filter state ----

const localFilter = reactive({
  year_min: -1, // -1 = 不限
  year_max: -1, // -1 = 不限
  r18: null as string | null,
  is_ai: null as boolean | null,
})

const r18Options = [
  { label: '显示', value: null },
  { label: '隐藏', value: 'hidden' },
  { label: '仅R18', value: 'only' },
]

const aiOptions = [
  { label: '显示', value: null },
  { label: '隐藏', value: false },
  { label: '仅AI', value: true },
]

// ---- Computed ----

const isDark = computed(() => store.colorScheme === 'dark')

const yearOptions = computed<number[]>(() => {
  if (!props.stats?.yearly_trend) return []
  return props.stats.yearly_trend
    .map(y => y.year)
    .sort((a, b) => b - a)
})

const hasActiveFilters = computed(() => {
  return localFilter.year_min !== -1
    || localFilter.year_max !== -1
    || localFilter.r18 !== null
    || localFilter.is_ai !== null
})

const r18Label = computed(() => {
  const opt = r18Options.find(o => o.value === localFilter.r18)
  return opt ? opt.label : ''
})

const aiLabel = computed(() => {
  const opt = aiOptions.find(o => o.value === localFilter.is_ai)
  return opt ? opt.label : ''
})

// ---- Methods ----

function applyFilter() {
  emit('applyFilter', {
    year_min: localFilter.year_min === -1 ? null : localFilter.year_min,
    year_max: localFilter.year_max === -1 ? null : localFilter.year_max,
    r18: localFilter.r18,
    is_ai: localFilter.is_ai,
  })
}

function clearYear() {
  localFilter.year_min = -1
  localFilter.year_max = -1
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') { emit('close') }
}

// ---- Lifecycle ----

watch(() => props.show, val => {
  if (val) {
    localFilter.year_min = -1
    localFilter.year_max = -1
    localFilter.r18 = null
    localFilter.is_ai = null
    window.addEventListener('keydown', handleKeyDown)
  } else {
    window.removeEventListener('keydown', handleKeyDown)
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
})
</script>
