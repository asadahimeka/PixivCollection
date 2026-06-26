<template>
  <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
    <!-- R18 Yearly Trend -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-sm font-bold text-gray-700 dark:text-gray-200">
          R18年度趋势
        </h3>
      </div>
      <Bar :data="r18ChartData" :options="r18ChartOptions" />
    </div>

    <!-- AI Yearly Trend -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-sm font-bold text-gray-700 dark:text-gray-200">
          AI年度趋势
        </h3>
      </div>
      <Bar :data="aiChartData" :options="aiChartOptions" />
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
  Title,
  Tooltip,
} from 'chart.js'
import { useStore } from '@/store'

const props = defineProps<{
  r18Trend: R18TrendItem[]
  aiTrend: AiTrendItem[]
}>()

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip, Legend, Title)

const store = useStore()

interface R18TrendItem {
  year: number
  x_restrict: number
  count: number
}

interface AiTrendItem {
  year: number
  is_ai: boolean
  count: number
}

const isDark = computed(() => store.colorScheme === 'dark')

const textColor = computed(() => (isDark.value ? '#d1d5db' : '#4b5563'))
const gridColor = computed(() => (isDark.value ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)'))

// ---- R18 stacked bar ----

const r18Years = computed(() => {
  const years = [...new Set(props.r18Trend.map(d => d.year))]
  return years.sort((a, b) => a - b)
})

function r18DataFor(xRestrict: number) {
  return r18Years.value.map(year => {
    const item = props.r18Trend.find(d => d.year === year && d.x_restrict === xRestrict)
    return item ? item.count : 0
  })
}

const r18ChartData = computed(() => ({
  labels: r18Years.value,
  datasets: [
    {
      label: '安全',
      data: r18DataFor(0),
      backgroundColor: 'rgba(34, 197, 94, 0.75)',
      borderColor: 'rgba(34, 197, 94, 1)',
      borderWidth: 1,
      borderRadius: 1,
    },
    {
      label: 'R18',
      data: r18DataFor(1),
      backgroundColor: 'rgba(249, 115, 22, 0.75)',
      borderColor: 'rgba(249, 115, 22, 1)',
      borderWidth: 1,
      borderRadius: 1,
    },
    {
      label: 'R18G',
      data: r18DataFor(2),
      backgroundColor: 'rgba(239, 68, 68, 0.75)',
      borderColor: 'rgba(239, 68, 68, 1)',
      borderWidth: 1,
      borderRadius: 1,
    },
  ],
}))

const r18ChartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: true,
  plugins: {
    legend: {
      labels: { color: textColor.value },
    },
    tooltip: {
      backgroundColor: isDark.value ? '#374151' : '#fff',
      titleColor: textColor.value,
      bodyColor: textColor.value,
      borderColor: isDark.value ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)',
      borderWidth: 1,
      padding: 10,
      mode: 'index' as const,
      intersect: false,
      callbacks: {
        label(ctx: any) {
          const yearTotal = (ctx.chart.data.datasets as any[])
            .reduce((sum: number, ds: any) => sum + (ds.data[ctx.dataIndex] || 0), 0)
          const pct = yearTotal > 0 ? ((ctx.parsed.y / yearTotal) * 100).toFixed(1) : '0.0'
          return ` ${ctx.dataset.label}: ${ctx.parsed.y} (${pct}%)`
        },
      },
    },
  },
  scales: {
    x: {
      stacked: true,
      title: {
        display: true,
        text: '年份',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: { color: textColor.value, font: { size: 11 } },
      grid: { color: gridColor.value },
    },
    y: {
      stacked: true,
      title: {
        display: true,
        text: '作品数',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: { color: textColor.value, font: { size: 11 } },
      grid: { color: gridColor.value },
      beginAtZero: true,
    },
  },
}))

// ---- AI stacked bar ----

const aiYears = computed(() => {
  const years = [...new Set(props.aiTrend.map(d => d.year))]
  return years.sort((a, b) => a - b)
})

function aiDataFor(isAi: boolean) {
  return aiYears.value.map(year => {
    const item = props.aiTrend.find(d => d.year === year && d.is_ai === isAi)
    return item ? item.count : 0
  })
}

const aiChartData = computed(() => ({
  labels: aiYears.value,
  datasets: [
    {
      label: '非AI',
      data: aiDataFor(false),
      backgroundColor: 'rgba(59, 130, 246, 0.75)',
      borderColor: 'rgba(59, 130, 246, 1)',
      borderWidth: 1,
      borderRadius: 1,
    },
    {
      label: 'AI',
      data: aiDataFor(true),
      backgroundColor: 'rgba(168, 85, 247, 0.75)',
      borderColor: 'rgba(168, 85, 247, 1)',
      borderWidth: 1,
      borderRadius: 1,
    },
  ],
}))

const aiChartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: true,
  plugins: {
    legend: {
      labels: { color: textColor.value },
    },
    tooltip: {
      backgroundColor: isDark.value ? '#374151' : '#fff',
      titleColor: textColor.value,
      bodyColor: textColor.value,
      borderColor: isDark.value ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)',
      borderWidth: 1,
      padding: 10,
      mode: 'index' as const,
      intersect: false,
      callbacks: {
        label(ctx: any) {
          const yearTotal = (ctx.chart.data.datasets as any[])
            .reduce((sum: number, ds: any) => sum + (ds.data[ctx.dataIndex] || 0), 0)
          const pct = yearTotal > 0 ? ((ctx.parsed.y / yearTotal) * 100).toFixed(1) : '0.0'
          return ` ${ctx.dataset.label}: ${ctx.parsed.y} (${pct}%)`
        },
      },
    },
  },
  scales: {
    x: {
      stacked: true,
      title: {
        display: true,
        text: '年份',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: { color: textColor.value, font: { size: 11 } },
      grid: { color: gridColor.value },
    },
    y: {
      stacked: true,
      title: {
        display: true,
        text: '作品数',
        color: textColor.value,
        font: { size: 12 },
      },
      ticks: { color: textColor.value, font: { size: 11 } },
      grid: { color: gridColor.value },
      beginAtZero: true,
    },
  },
}))
</script>
