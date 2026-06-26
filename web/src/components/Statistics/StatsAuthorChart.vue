<template>
  <div class="space-y-6">
    <!-- Chart Section -->
    <div>
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-base font-bold text-gray-800 dark:text-gray-200">
          作者排行 TOP20
        </h3>
        <!-- Sort toggle -->
        <div class="flex overflow-hidden rounded-md border dark:border-white/20">
          <button
            v-for="opt in sortOptions" :key="opt.value"
            class="px-2 py-1 text-xs transition-colors"
            :class="sortMode === opt.value
              ? 'bg-blue-500 text-white'
              : 'hover:bg-gray-100 dark:hover:bg-white/10'"
            @click="sortMode = opt.value"
          >
            {{ opt.label }}
          </button>
        </div>
      </div>
      <div class="relative" style="height: 420px">
        <Bar
          v-if="top20.length > 0"
          :data="chartData"
          :options="chartOptions"
        />
        <div
          v-else
          class="flex h-full items-center justify-center text-gray-400 dark:text-gray-500"
        >
          暂无数据
        </div>
      </div>
    </div>

    <!-- Table Section -->
    <div>
      <h3 class="mb-4 text-base font-bold text-gray-800 dark:text-gray-200">
        作者排行 TOP200
      </h3>
      <div class="max-h-64 overflow-y-auto rounded-lg border dark:border-white/10">
        <table class="w-full text-sm">
          <thead>
            <tr class="bg-gray-50 text-gray-600 dark:bg-white/5 dark:text-gray-400">
              <th class="w-12 px-3 py-2 text-center font-medium">
                排名
              </th>
              <th class="px-3 py-2 text-left font-medium">
                作者
              </th>
              <th class="w-20 px-3 py-2 text-right font-medium">
                作品数
              </th>
              <th class="w-24 px-3 py-2 text-right font-medium">
                总收藏
              </th>
              <th class="w-24 px-3 py-2 text-right font-medium">
                总浏览
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(author, index) in top200"
              :key="author.author_id"
              class="border-t border-gray-100 transition-colors hover:bg-blue-50/50 dark:border-white/5 dark:hover:bg-blue-900/10"
              :class="index % 2 === 0
                ? 'bg-white dark:bg-transparent'
                : 'bg-gray-50/50 dark:bg-white/[0.03]'"
            >
              <td class="px-3 py-2 text-center text-gray-500 dark:text-gray-400">
                {{ index + 1 }}
              </td>
              <td class="max-w-0 px-3 py-2">
                <span
                  class="block cursor-pointer truncate text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                  :title="author.author_name"
                  @click="emit('viewAuthor', author.author_id)"
                >
                  {{ author.author_name || '(佚名)' }}
                </span>
              </td>
              <td class="px-3 py-2 text-right font-medium tabular-nums">
                {{ author.illustration_count }}
              </td>
              <td class="px-3 py-2 text-right tabular-nums text-gray-600 dark:text-gray-400">
                {{ formatNumber(author.total_bookmarks) }}
              </td>
              <td class="px-3 py-2 text-right tabular-nums text-gray-600 dark:text-gray-400">
                {{ formatNumber(author.total_views) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Bar } from 'vue-chartjs'
import { Chart as ChartJS, registerables } from 'chart.js'
import { useStore } from '@/store'

const props = defineProps<{
  authors: AuthorStats[]
}>()

const emit = defineEmits<{
  viewAuthor: [id: number]
}>()

ChartJS.register(...registerables)

interface AuthorStats {
  author_id: number
  author_name: string
  author_account: string
  illustration_count: number
  total_bookmarks: number
  total_views: number
}

const store = useStore()

const isDark = computed(() => store.colorScheme === 'dark')

const sortMode = ref<'illustration_count' | 'total_bookmarks'>('illustration_count')

const sortOptions = [
  { label: '按作品数', value: 'illustration_count' as const },
  { label: '按总收藏', value: 'total_bookmarks' as const },
]

const sortedAuthors = computed(() => {
  return [...props.authors].sort((a, b) => b[sortMode.value] - a[sortMode.value])
})

const top20 = computed(() => sortedAuthors.value.slice(0, 20))
const top200 = computed(() => sortedAuthors.value.slice(0, 200))

function formatNumber(n: number): string {
  return n.toLocaleString()
}

function generateGradientColors(count: number): string[] {
  return Array.from({ length: count }, (_, i) => {
    const t = count > 1 ? i / (count - 1) : 0
    const hue = 225 + t * 55
    const sat = 80
    const lig = 58
    return `hsla(${hue}, ${sat}%, ${lig}%, 0.75)`
  })
}

const textColor = computed(() =>
  isDark.value ? 'rgba(255, 255, 255, 0.7)' : 'rgba(0, 0, 0, 0.7)',
)
const gridColor = computed(() =>
  isDark.value ? 'rgba(255, 255, 255, 0.06)' : 'rgba(0, 0, 0, 0.06)',
)

const chartData = computed(() => {
  const colors = generateGradientColors(20)
  const label = sortMode.value === 'illustration_count' ? '作品数' : '总收藏'
  return {
    labels: top20.value.map(a => a.author_name),
    datasets: [
      {
        label,
        data: top20.value.map(a => a[sortMode.value]),
        // bars ordered rank20→rank1 (top→bottom), gradient blue→purple (small→large)
        backgroundColor: colors,
        borderColor: colors.map((c: string) => c.replace('0.75', '1')),
        borderWidth: 1,
        borderRadius: 3,
      },
    ],
  }
})

const chartOptions = computed(() => ({
  indexAxis: 'y' as const,
  responsive: true,
  maintainAspectRatio: false,
  animation: {
    duration: 800,
    easing: 'easeOutQuart' as const,
  },
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: isDark.value ? 'rgba(30, 30, 30, 0.95)' : 'rgba(255, 255, 255, 0.95)',
      titleColor: isDark.value ? 'rgba(255, 255, 255, 0.9)' : 'rgba(0, 0, 0, 0.9)',
      bodyColor: isDark.value ? 'rgba(255, 255, 255, 0.7)' : 'rgba(0, 0, 0, 0.7)',
      borderColor: isDark.value ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)',
      borderWidth: 1,
      padding: 10,
      cornerRadius: 8,
      displayColors: false,
      callbacks: {
        title(items: any[]) {
          return items[0]?.label ?? ''
        },
        label(ctx: any) {
          const name = ctx.chart.data.labels[ctx.dataIndex] || '(佚名)'
          return `${name}: ${ctx.parsed.x}`
        },
      },
    },
  },
  scales: {
    x: {
      title: {
        display: true,
        text: sortMode.value === 'illustration_count' ? '作品数' : '总收藏',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: {
        color: textColor.value,
        font: { size: 11 },
        stepSize: 1,
      },
      grid: {
        color: gridColor.value,
      },
      border: {
        color: gridColor.value,
      },
    },
    y: {
      title: {
        display: true,
        text: '作者',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: {
        color: textColor.value,
        font: { size: 11 },
      },
      grid: {
        display: false,
      },
      border: {
        color: gridColor.value,
      },
    },
  },
}))
</script>
