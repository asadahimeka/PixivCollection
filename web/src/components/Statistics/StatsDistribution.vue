<template>
  <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
    <!-- Bookmark Distribution -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-sm font-bold text-gray-700 dark:text-gray-200">
          收藏分布
        </h3>
        <span class="text-xs text-gray-400 dark:text-gray-500">
          共 {{ totalBookmark }} 作品
        </span>
      </div>
      <Bar :data="bookmarkChartData" :options="chartOptions" />
    </div>

    <!-- View Distribution -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-sm font-bold text-gray-700 dark:text-gray-200">
          浏览分布
        </h3>
        <span class="text-xs text-gray-400 dark:text-gray-500">
          共 {{ totalView }} 作品
        </span>
      </div>
      <Bar :data="viewChartData" :options="chartOptions" />
    </div>
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
  Tooltip,
} from 'chart.js'
import { useStore } from '@/store'

const props = defineProps<{
  bookmarkDist: DistributionBucket[]
  viewDist: DistributionBucket[]
}>()

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip, Legend)

interface DistributionBucket {
  label: string
  min: number
  max: number | null
  count: number
}

const store = useStore()
const isDark = computed(() => store.colorScheme === 'dark')

const textColor = computed(() => (isDark.value ? '#d1d5db' : '#4b5563'))
const gridColor = computed(() => (isDark.value ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)'))

const totalBookmark = computed(() => props.bookmarkDist.reduce((a, b) => a + b.count, 0))
const totalView = computed(() => props.viewDist.reduce((a, b) => a + b.count, 0))

const bookmarkChartData = computed(() => ({
  labels: props.bookmarkDist.map(d => d.label),
  datasets: [
    {
      label: '作品数',
      data: props.bookmarkDist.map(d => d.count),
      backgroundColor: 'rgba(59, 130, 246, 0.7)',
      borderColor: 'rgba(59, 130, 246, 1)',
      borderWidth: 1,
      borderRadius: 4,
      hoverBackgroundColor: 'rgba(59, 130, 246, 0.9)',
    },
  ],
}))

const viewChartData = computed(() => ({
  labels: props.viewDist.map(d => d.label),
  datasets: [
    {
      label: '作品数',
      data: props.viewDist.map(d => d.count),
      backgroundColor: 'rgba(168, 85, 247, 0.7)',
      borderColor: 'rgba(168, 85, 247, 1)',
      borderWidth: 1,
      borderRadius: 4,
      hoverBackgroundColor: 'rgba(168, 85, 247, 0.9)',
    },
  ],
}))

const chartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: true,
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: isDark.value ? '#374151' : '#fff',
      titleColor: textColor.value,
      bodyColor: textColor.value,
      borderColor: isDark.value ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)',
      borderWidth: 1,
      padding: 10,
      callbacks: {
        label: (ctx: any) => {
          const total = (ctx.dataset.data as number[]).reduce((a: number, b: number) => a + b, 0)
          const pct = total > 0 ? ((ctx.parsed.y / total) * 100).toFixed(1) : '0'
          return ` ${ctx.parsed.y} 作品 (${pct}%)`
        },
      },
    },
  },
  scales: {
    x: {
      title: {
        display: true,
        text: '区间',
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
    },
    y: {
      title: {
        display: true,
        text: '作品数',
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
      beginAtZero: true,
    },
  },
}))
</script>
