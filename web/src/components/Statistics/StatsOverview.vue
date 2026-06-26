<template>
  <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
    <div class="rounded-xl bg-blue-50 p-4 dark:bg-blue-900/20">
      <div class="text-3xl font-bold text-blue-600 dark:text-blue-400">
        {{ formatNumber(overview.total_illustrations) }}
      </div>
      <div class="mt-1 text-sm text-gray-500 dark:text-gray-400">
        总作品数
      </div>
    </div>
    <div class="rounded-xl bg-green-50 p-4 dark:bg-green-900/20">
      <div class="text-3xl font-bold text-green-600 dark:text-green-400">
        {{ formatNumber(overview.total_authors) }}
      </div>
      <div class="mt-1 text-sm text-gray-500 dark:text-gray-400">
        总作者数
      </div>
    </div>
    <div class="rounded-xl bg-purple-50 p-4 dark:bg-purple-900/20">
      <div class="text-3xl font-bold text-purple-600 dark:text-purple-400">
        {{ formatNumber(overview.total_tags) }}
      </div>
      <div class="mt-1 text-sm text-gray-500 dark:text-gray-400">
        总标签数
      </div>
    </div>
    <div class="rounded-xl bg-amber-50 p-4 dark:bg-amber-900/20">
      <div class="text-3xl font-bold text-amber-600 dark:text-amber-400">
        {{ overview.year_range[0] }} - {{ overview.year_range[1] }}
      </div>
      <div class="mt-1 text-sm text-gray-500 dark:text-gray-400">
        创作年份
      </div>
    </div>
  </div>

  <!-- Insight paragraph -->
  <div
    v-if="insightText"
    class="mt-6 rounded-xl bg-gradient-to-r from-blue-50 to-purple-50 p-4 text-sm leading-relaxed text-gray-700 dark:from-blue-900/20 dark:to-purple-900/20 dark:text-gray-300"
  >
    {{ insightText }}
  </div>
</template>

<script setup lang="ts">
interface AuthorStats {
  author_id: number
  author_name: string
  author_account: string
  illustration_count: number
  total_bookmarks: number
  total_views: number
}

interface R18Ratio {
  safe: number
  r18: number
  r18g: number
}

interface AiRatio {
  ai: number
  non_ai: number
}

const props = defineProps<{
  overview: {
    total_illustrations: number
    total_authors: number
    total_tags: number
    year_range: [number, number]
  }
  authorRanking?: AuthorStats[]
  r18Ratio?: R18Ratio
  aiRatio?: AiRatio
}>()

const insightText = computed(() => {
  const { overview, authorRanking, r18Ratio, aiRatio } = props
  if (!authorRanking || authorRanking.length === 0 || !r18Ratio || !aiRatio) { return '' }

  const top = authorRanking[0]
  const totalR18 = r18Ratio.safe + r18Ratio.r18 + r18Ratio.r18g
  const r18Pct = totalR18 > 0 ? ((r18Ratio.r18 + r18Ratio.r18g) / totalR18 * 100).toFixed(1) : '0'
  const totalAi = aiRatio.ai + aiRatio.non_ai
  const aiPct = totalAi > 0 ? (aiRatio.ai / totalAi * 100).toFixed(1) : '0'
  const yearMin = overview.year_range[0]
  const yearMax = overview.year_range[1]
  const span = yearMax - yearMin + 1

  return `你的收藏横跨 ${yearMin}-${yearMax} 共 ${span} 年，最爱的画师是 ${top.author_name}，收藏了他的 ${top.illustration_count} 件作品。收藏中 R18 占 ${r18Pct}%，AI 作品占 ${aiPct}%。`
})

function formatNumber(n: number): string {
  return n.toLocaleString()
}
</script>
