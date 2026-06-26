<template>
  <div class="space-y-4">
    <!-- Chart: TOP 30 -->
    <div
      v-if="tags.length > 0"
      class="rounded-xl bg-white p-4 dark:bg-[#242424]"
    >
      <h3 class="mb-4 text-sm font-medium text-gray-500 dark:text-gray-400">
        标签排行 TOP30
      </h3>
      <div class="relative" style="height: 420px">
        <Bar
          v-if="chartData"
          ref="chartRef"
          :data="chartData"
          :options="chartOptions"
        />
      </div>
    </div>

    <!-- Ranking table (all tags) -->
    <div class="rounded-xl bg-white dark:bg-[#242424]">
      <div class="max-h-80 overflow-y-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b dark:border-white/20">
              <th class="sticky top-0 z-10 bg-white px-4 py-2.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:bg-[#242424] dark:text-gray-400">
                排名
              </th>
              <th class="sticky top-0 z-10 bg-white px-4 py-2.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:bg-[#242424] dark:text-gray-400">
                 原名
              </th>
              <th class="sticky top-0 z-10 bg-white px-4 py-2.5 text-left text-xs font-medium uppercase tracking-wider text-gray-500 dark:bg-[#242424] dark:text-gray-400">
                 译名
              </th>
              <th class="sticky top-0 z-10 bg-white px-4 py-2.5 text-right text-xs font-medium uppercase tracking-wider text-gray-500 dark:bg-[#242424] dark:text-gray-400">
                出现次数
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(tag, index) in tags"
              :key="tag.name"
              class="border-b transition-colors hover:bg-green-50/50 dark:border-white/10 dark:hover:bg-green-900/10"
              :class="index % 2 === 0 ? '' : 'bg-gray-50/50 dark:bg-white/[0.02]'"
            >
              <td class="px-4 py-2 text-gray-500 dark:text-gray-400">
                {{ index + 1 }}
              </td>
              <td
                class="cursor-pointer px-4 py-2 text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                @click="emit('viewTag', tag.name)"
              >
                {{ tag.name }}
              </td>
              <td class="px-4 py-2 text-gray-500 dark:text-gray-400">
                {{ tag.translated_name || '-' }}
              </td>
              <td class="px-4 py-2 text-right font-medium tabular-nums text-gray-900 dark:text-gray-100">
                {{ formatCount(tag.count) }}
              </td>
            </tr>
            <tr v-if="tags.length === 0">
              <td
                colspan="4"
                class="px-4 py-8 text-center text-gray-400"
              >
                暂无标签数据
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
import { type Chart, Chart as ChartJS, registerables } from 'chart.js'
import { useStore } from '@/store'

const props = defineProps<{
  tags: TagStats[]
}>()

const emit = defineEmits<{
  viewTag: [name: string]
}>()

ChartJS.register(...registerables)

interface TagStats {
  name: string
  translated_name: string | null
  count: number
}

const store = useStore()
const chartRef = ref<{ chart: Chart<'bar'> }>()

const isDark = computed(() => store.colorScheme === 'dark')

const chartData = computed(() => {
  const top = props.tags.slice(0, 30)
  const labels = top.map(t => t.name)
  const counts = top.map(t => t.count)

  return {
    labels,
    datasets: [
      {
        data: counts,
        backgroundColor: 'rgba(34, 197, 94, 0.7)',
        borderColor: 'rgba(34, 197, 94, 1)',
        borderWidth: 1,
        borderRadius: 4,
        hoverBackgroundColor: 'rgba(34, 197, 94, 0.9)',
      },
    ],
  }
})

const chartOptions = computed(() => ({
  indexAxis: 'y' as const,
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { display: false },
    tooltip: {
      backgroundColor: isDark.value ? '#333' : '#fff',
      titleColor: isDark.value ? '#fff' : '#111',
      bodyColor: isDark.value ? '#ccc' : '#333',
      borderColor: isDark.value ? '#444' : '#ddd',
      borderWidth: 1,
      padding: 8,
      cornerRadius: 6,
      displayColors: false,
    },
  },
  scales: {
    x: {
      beginAtZero: true,
      title: {
        display: true,
        text: '出现次数',
        color: isDark.value ? '#9ca3af' : '#6b7280',
        font: { size: 12 },
      },
      grid: {
        color: isDark.value ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.06)',
      },
      ticks: {
        color: isDark.value ? '#9ca3af' : '#6b7280',
        font: { size: 11 },
      },
    },
    y: {
      grid: { display: false },
      ticks: {
        color: isDark.value ? '#d1d5db' : '#374151',
        font: { size: 11 },
      },
    },
  },
}))

function formatCount(n: number): string {
  return n.toLocaleString()
}

watch(() => store.colorScheme, () => {
  nextTick(() => {
    chartRef.value?.chart?.update()
  })
})
</script>
