<template>
  <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
    <!-- R18 Distribution -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
        R18分布
      </h3>
      <Pie :data="r18ChartData" :options="chartOptions" />
    </div>

    <!-- AI Distribution -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
        AI分布
      </h3>
      <Pie :data="aiChartData" :options="chartOptions" />
    </div>

    <!-- Shape Distribution -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
        形状分布
      </h3>
      <Doughnut :data="shapeChartData" :options="chartOptions" />
    </div>
  </div>

  <!-- Sanity Level Bar Chart -->
  <div
    v-if="sanityLevels && sanityLevels.length > 0"
    class="mt-6 rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]"
  >
    <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
      健全度分布
    </h3>
    <div style="height: 260px">
      <Bar :data="sanityChartData" :options="sanityChartOptions" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Bar, Doughnut, Pie } from 'vue-chartjs'
import {
  ArcElement,
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Legend,
  LinearScale,
  Title,
  Tooltip,
} from 'chart.js'
import { useStore } from '@/store'
import { FILTER_SHAPES } from '@/config'

const props = defineProps<{
  r18Ratio: R18Ratio
  aiRatio: AiRatio
  shapeDist: ShapeBucket[]
  sanityLevels?: SanityLevelBucket[]
}>()

ChartJS.register(ArcElement, Legend, Tooltip, BarElement, CategoryScale, LinearScale, Title)

const store = useStore()

interface R18Ratio {
  safe: number
  r18: number
  r18g: number
}

interface AiRatio {
  ai: number
  non_ai: number
}

interface ShapeBucket {
  shape: string
  count: number
}

interface SanityLevelBucket {
  level: number
  count: number
}

const isDark = computed(() => store.colorScheme === 'dark')

const textColor = computed(() => (isDark.value ? '#d1d5db' : '#4b5563'))
const borderColor = computed(() => (isDark.value ? '#242424' : '#fff'))

function shapeLabel(shape: string): string {
  if (shape === 'other') return '其他'
  return FILTER_SHAPES[shape] || shape
}

const shapeColorPalette = [
  '#3b82f6', '#22c55e', '#f97316', '#a855f7',
  '#ec4899', '#06b6d4', '#eab308', '#8b5cf6',
  '#14b8a6', '#f43f5e',
]

const r18ChartData = computed(() => ({
  labels: ['健全', 'R18', 'R18G'],
  datasets: [
    {
      data: [props.r18Ratio.safe, props.r18Ratio.r18, props.r18Ratio.r18g],
      backgroundColor: ['#22c55e', '#f97316', '#ef4444'],
      borderColor: borderColor.value,
      borderWidth: 2,
      hoverOffset: 4,
    },
  ],
}))

const aiChartData = computed(() => ({
  labels: ['非AI', 'AI'],
  datasets: [
    {
      data: [props.aiRatio.non_ai, props.aiRatio.ai],
      backgroundColor: ['#3b82f6', '#a855f7'],
      borderColor: borderColor.value,
      borderWidth: 2,
      hoverOffset: 4,
    },
  ],
}))

const shapeChartData = computed(() => ({
  labels: props.shapeDist.map(s => shapeLabel(s.shape)),
  datasets: [
    {
      data: props.shapeDist.map(s => s.count),
      backgroundColor: props.shapeDist.map((_, i) => shapeColorPalette[i % shapeColorPalette.length]),
      borderColor: borderColor.value,
      borderWidth: 2,
      hoverOffset: 4,
    },
  ],
}))

const chartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: true,
  plugins: {
    legend: {
      position: 'bottom' as const,
      labels: {
        color: textColor.value,
        padding: 12,
        usePointStyle: true,
        font: { size: 11 },
      },
    },
    tooltip: {
      backgroundColor: isDark.value ? '#374151' : '#fff',
      titleColor: textColor.value,
      bodyColor: textColor.value,
      borderColor: isDark.value ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)',
      borderWidth: 1,
      padding: 10,
      callbacks: {
        label: (ctx: any) => {
          const dataset = ctx.dataset.data as number[]
          const total = dataset.reduce((a: number, b: number) => a + b, 0)
          const pct = total > 0 ? ((ctx.parsed / total) * 100).toFixed(1) : '0'
          return ` ${ctx.label}: ${ctx.parsed} (${pct}%)`
        },
      },
    },
  },
}))

function sanityColor(level: number): string {
  const t = Math.min(level / 6, 1)
  const r = Math.round(t * 255)
  const g = Math.round((1 - t) * 255)
  return `rgb(${r}, ${g}, 50)`
}

const sanityChartData = computed(() => ({
  labels: (props.sanityLevels ?? []).map(s => String(s.level)),
  datasets: [
    {
      label: '作品数',
      data: (props.sanityLevels ?? []).map(s => s.count),
      backgroundColor: (props.sanityLevels ?? []).map(s => sanityColor(s.level)),
      borderColor: (props.sanityLevels ?? []).map(s => sanityColor(s.level)),
      borderWidth: 1,
      borderRadius: 3,
    },
  ],
}))

const sanityChartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: isDark.value ? '#374151' : '#fff',
      titleColor: textColor.value,
      bodyColor: textColor.value,
      borderColor: isDark.value ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)',
      borderWidth: 1,
      padding: 10,
    },
  },
  scales: {
    x: {
      title: {
        display: true,
        text: '健全度等级',
        color: textColor.value,
        font: { size: 11 },
      },
      ticks: { color: textColor.value },
      grid: { display: false },
    },
    y: {
      beginAtZero: true,
      ticks: {
        color: textColor.value,
        precision: 0,
      },
      grid: {
        color: isDark.value ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)',
      },
    },
  },
}))
</script>
