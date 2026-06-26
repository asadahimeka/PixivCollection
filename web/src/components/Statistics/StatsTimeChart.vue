<template>
  <div class="rounded-xl bg-white p-4 dark:bg-[#1a1a1a]">
    <Bar :data="chartData" :options="chartOptions" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Bar } from 'vue-chartjs'
import {
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Filler,
  Legend,
  LineElement,
  LinearScale,
  PointElement,
  Title,
  Tooltip,
} from 'chart.js'
import { useStore } from '@/store'

const props = defineProps<{
  yearly: YearlyStats[]
}>()

ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  LineElement,
  PointElement,
  Legend,
  Title,
  Tooltip,
  Filler,
)

const store = useStore()

interface YearlyStats {
  year: number
  count: number
  avg_bookmark: number
  avg_view: number
}

const isDark = computed(() => store.colorScheme === 'dark')

const textColor = computed(() => (isDark.value ? '#d1d5db' : '#6b7280'))

const gridColor = computed(() => (isDark.value ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)'))

const cumulativeValues = computed(() => {
  let sum = 0
  return props.yearly.map(y => {
    sum += y.count
    return sum
  })
})

const chartData = computed(() => ({
  labels: props.yearly.map(y => y.year),
  datasets: [
    {
      label: '作品数',
      type: 'bar' as const,
      data: props.yearly.map(y => y.count),
      backgroundColor: 'rgba(59, 130, 246, 0.6)',
      borderColor: 'rgba(59, 130, 246, 1)',
      borderWidth: 1,
      yAxisID: 'y',
      order: 2,
    },
    {
      label: '平均收藏',
      type: 'line' as any,
      data: props.yearly.map(y => Math.round(y.avg_bookmark)),
      borderColor: '#f97316',
      backgroundColor: 'rgba(249, 115, 22, 0.1)',
      pointBackgroundColor: '#f97316',
      tension: 0.3,
      yAxisID: 'y1',
      order: 1,
    },
    {
      label: '累计作品',
      type: 'line' as any,
      data: cumulativeValues.value,
      borderColor: '#22c55e',
      backgroundColor: 'rgba(34, 197, 94, 0.1)',
      pointBackgroundColor: '#22c55e',
      fill: true,
      tension: 0.3,
      yAxisID: 'y',
      order: 0,
    },
  ],
}))

const chartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: true,
  interaction: {
    mode: 'index' as const,
    intersect: false,
  },
  plugins: {
    legend: {
      labels: {
        color: textColor.value,
      },
    },
    title: {
      display: true,
      text: '年度趋势',
      color: textColor.value,
      font: { size: 14 },
    },
  },
  scales: {
    x: {
      title: {
        display: true,
        text: '年份',
        color: textColor.value,
      },
      ticks: { color: textColor.value },
      grid: { color: gridColor.value },
    },
    y: {
      type: 'linear' as const,
      position: 'left' as const,
      title: {
        display: true,
        text: '作品数',
        color: textColor.value,
      },
      ticks: { color: textColor.value },
      grid: { color: gridColor.value },
      beginAtZero: true,
    },
    y1: {
      type: 'linear' as const,
      position: 'right' as const,
      title: {
        display: true,
        text: '平均收藏',
        color: textColor.value,
      },
      ticks: { color: textColor.value },
      grid: {
        drawOnChartArea: false,
        color: gridColor.value,
      },
      beginAtZero: true,
    },
  },
}))
</script>
