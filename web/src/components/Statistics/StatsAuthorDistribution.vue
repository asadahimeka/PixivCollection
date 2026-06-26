<template>
  <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
    <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
      作者作品数分布
    </h3>
    <div class="relative" style="height: 320px">
      <Bar
        v-if="authorDist.length > 0"
        ref="chartRef"
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
    <p class="mt-4 text-center text-sm text-gray-500 dark:text-gray-400">
      你有 {{ totalAuthors }} 位作者的作品被收藏，其中 {{ topAuthorCount }} 位作者贡献了超过 20 件作品
    </p>
  </div>
</template>

<script setup lang="ts">
import { Bar } from 'vue-chartjs'
import {
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Legend,
  LinearScale,
  Title,
  Tooltip,
} from 'chart.js'
import type { Chart } from 'chart.js'
import { useStore } from '@/store'

const props = defineProps<{
  authorDist: AuthorBucket[]
}>()

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip, Legend, Title)

interface AuthorBucket {
  label: string
  count: number
}

const store = useStore()
const chartRef = ref<{ chart: Chart<'bar'> }>()

const isDark = computed(() => store.colorScheme === 'dark')

const totalAuthors = computed(() =>
  props.authorDist.reduce((sum, b) => sum + b.count, 0),
)

const topAuthorCount = computed(() => {
  const last = props.authorDist[props.authorDist.length - 1]
  return last ? last.count : 0
})

const sortedDist = computed(() =>
  [...props.authorDist].sort((a, b) => a.count - b.count),
)

const textColor = computed(() =>
  isDark.value ? 'rgba(255, 255, 255, 0.7)' : 'rgba(0, 0, 0, 0.7)',
)

const gridColor = computed(() =>
  isDark.value ? 'rgba(255, 255, 255, 0.06)' : 'rgba(0, 0, 0, 0.06)',
)

function generateColors(count: number): { bg: string[]; border: string[]; hover: string[] } {
  const bg: string[] = []
  const border: string[] = []
  const hover: string[] = []
  for (let i = 0; i < count; i++) {
    const t = count > 1 ? i / (count - 1) : 0
    // blue(225) -> purple(280) gradient
    const hue = 225 + t * 55
    bg.push(`hsla(${hue}, 80%, 58%, 0.75)`)
    border.push(`hsla(${hue}, 80%, 58%, 1)`)
    hover.push(`hsla(${hue}, 80%, 58%, 0.9)`)
  }
  return { bg, border, hover }
}

const chartData = computed(() => {
  const sorted = sortedDist.value
  const count = sorted.length
  const labels = sorted.map(b => b.label)
  const { bg, border, hover } = generateColors(count)

  return {
    labels,
    datasets: [
      {
        label: '作者数',
        data: sorted.map(b => b.count),
        backgroundColor: bg,
        borderColor: border,
        borderWidth: 1,
        borderRadius: 4,
        hoverBackgroundColor: hover,
      },
    ],
  }
})

const chartOptions = computed(() => ({
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
        label(ctx: any) {
          return `作者数: ${ctx.parsed.y}`
        },
      },
    },
  },
  scales: {
    x: {
      title: {
        display: true,
        text: '作品数区间',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: {
        color: textColor.value,
        font: { size: 11 },
      },
      grid: {
        color: gridColor.value,
        display: true,
      },
      border: {
        color: gridColor.value,
      },
    },
    y: {
      title: {
        display: true,
        text: '作者数',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: {
        color: textColor.value,
        font: { size: 11 },
      },
      grid: {
        color: gridColor.value,
      },
      border: {
        color: gridColor.value,
      },
      beginAtZero: true,
    },
  },
}))

watch(() => store.colorScheme, () => {
  nextTick(() => {
    chartRef.value?.chart?.update()
  })
})
</script>
