<template>
  <div class="rounded-xl bg-white p-4 shadow-sm dark:bg-[#242424]">
    <div class="mb-4 flex items-center justify-between">
      <h3 class="text-sm font-bold text-gray-700 dark:text-gray-200">
        标签年度趋势
      </h3>
      <span class="text-xs text-gray-400 dark:text-gray-500">TOP20 标签</span>
    </div>
    <div class="relative" style="height: 400px">
      <canvas ref="canvasRef"></canvas>
    </div>
    <div class="mt-3 flex flex-wrap gap-x-4 gap-y-1.5">
      <button
        v-for="(tag, i) in topTags"
        :key="tag"
        class="flex items-center gap-1.5 text-xs transition-opacity hover:opacity-80"
        :class="hiddenTags.has(tag) ? 'opacity-40' : 'opacity-90'"
        @click="toggleTag(tag)"
      >
        <span
          class="inline-block h-2.5 w-2.5 rounded-sm"
          :style="{ backgroundColor: TAG_COLORS[i % TAG_COLORS.length].solid }"
        ></span>
        <span class="text-gray-500 dark:text-gray-400">{{ tag }}</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Chart as ChartJS, registerables } from 'chart.js'
import { useStore } from '@/store'

const props = defineProps<{
  tagTrend: TagTrendItem[]
}>()

ChartJS.register(...registerables)

interface TagTrendItem {
  year: number
  tag_name: string
  count: number
}

const store = useStore()
const canvasRef = ref<HTMLCanvasElement>()
let chartInstance: ChartJS | null = null
const hiddenTags = ref<Set<string>>(new Set())

const isDark = computed(() => store.colorScheme === 'dark')

// Golden-angle-spaced hues for 20 max tags
const TAG_COLORS = Array.from({ length: 20 }, (_, i) => {
  const hue = (i * 137.508) % 360
  return {
    fill: `hsla(${hue.toFixed(1)}, 58%, 55%, 0.25)`,
    border: `hsla(${hue.toFixed(1)}, 58%, 48%, 0.7)`,
    solid: `hsl(${hue.toFixed(1)}, 58%, 55%)`,
  }
})

const years = computed(() => {
  const yrs = [...new Set(props.tagTrend.map(d => d.year))]
  return yrs.sort((a, b) => a - b)
})

const topTags = computed(() => {
  const totals = new Map<string, number>()
  for (const item of props.tagTrend) {
    if (item.count <= 0) continue
    totals.set(item.tag_name, (totals.get(item.tag_name) ?? 0) + item.count)
  }
  return [...totals.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 20)
    .map(([name]) => name)
})

const chartData = computed(() => {
  const tags = topTags.value
  const labels = years.value
  const isHidden = hiddenTags.value

  return {
    labels,
    datasets: tags.map((tag, i) => {
      const data = labels.map(year => {
        const item = props.tagTrend.find(d => d.year === year && d.tag_name === tag)
        return item ? item.count : 0
      })
      const color = TAG_COLORS[i % TAG_COLORS.length]
      return {
        label: tag,
        data,
        fill: i === 0 ? 'origin' as const : '-1' as const,
        backgroundColor: color.fill,
        borderColor: color.border,
        borderWidth: 1,
        pointRadius: 0,
        pointHoverRadius: 3,
        tension: 0.3,
        hidden: isHidden.has(tag),
      }
    }),
  }
})

const textColor = computed(() => (isDark.value ? '#d1d5db' : '#6b7280'))
const gridColor = computed(() => (isDark.value ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)'))

const chartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    mode: 'index' as const,
    intersect: false,
  },
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: isDark.value ? '#374151' : '#fff',
      titleColor: isDark.value ? '#f9fafb' : '#111',
      bodyColor: isDark.value ? '#d1d5db' : '#374151',
      borderColor: isDark.value ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.1)',
      borderWidth: 1,
      padding: 10,
      cornerRadius: 6,
      mode: 'index' as const,
      intersect: false,
      callbacks: {
        title(items: any[]) {
          if (items.length === 0) return ''
          return `${items[0].label}年`
        },
        label(ctx: any) {
          if (ctx.parsed.y === 0) return
          return ` ${ctx.dataset.label}: ${ctx.parsed.y}`
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
      ticks: {
        color: textColor.value,
        font: { size: 11 },
      },
      grid: {
        color: gridColor.value,
      },
    },
    y: {
      stacked: true,
      title: {
        display: true,
        text: '出现次数',
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

function toggleTag(tagName: string) {
  const next = new Set(hiddenTags.value)
  if (next.has(tagName)) { next.delete(tagName) } else { next.add(tagName) }
  hiddenTags.value = next
  applyHidden()
}

function applyHidden() {
  if (!chartInstance) return
  const tags = topTags.value
  tags.forEach((tag, i) => {
    const meta = chartInstance!.getDatasetMeta(i)
    if (meta) { meta.hidden = hiddenTags.value.has(tag) }
  })
  chartInstance.update()
}

function syncChart() {
  if (!chartInstance) return
  chartInstance.data = chartData.value
  chartInstance.options = chartOptions.value
  chartInstance.update()
}

watch(() => store.colorScheme, () => {
  nextTick(() => syncChart())
})

watch([years, topTags], () => {
  nextTick(() => syncChart())
}, { deep: true })

onMounted(() => {
  if (canvasRef.value) {
    chartInstance = new ChartJS(canvasRef.value, {
      type: 'line',
      data: chartData.value,
      options: chartOptions.value,
    })
  }
})

onUnmounted(() => {
  chartInstance?.destroy()
  chartInstance = null
})
</script>
