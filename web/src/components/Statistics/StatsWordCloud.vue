<template>
  <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
    <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
      标签词云
    </h3>
    <div
      ref="containerRef"
      class="relative flex items-center justify-center"
      :style="{ height: `${containerHeight}px` }"
    >
      <svg
        v-if="svgSize.width > 0"
        :width="svgSize.width"
        :height="svgSize.height"
        class="overflow-visible"
      >
        <text
          v-for="w in layoutWords"
          :key="w.text"
          :x="w.x"
          :y="w.y"
          :font-size="w.size"
          :fill="w.color"
          :transform="`rotate(${w.rotate}, ${w.x}, ${w.y})`"
          text-anchor="middle"
          class="cursor-pointer transition-opacity hover:opacity-70"
          :style="{ fontFamily: 'system-ui, sans-serif' }"
          @click="emit('viewTag', w.text)"
        >
          <title>{{ displayTagMap[w.text]?.translated_name || w.text }} ({{ displayTagMap[w.text]?.count || 0 }})</title>
          {{ w.text }}
        </text>
      </svg>
    </div>
    <div
      v-if="displayTags.length === 0"
      class="flex items-center justify-center py-12 text-sm text-gray-400 dark:text-gray-500"
    >
      暂无标签数据
    </div>
  </div>
</template>

<script setup lang="ts">
import cloud from 'd3-cloud'
import { useStore } from '@/store'

interface TagStats {
  name: string
  translated_name: string | null
  count: number
}

const props = defineProps<{
  tags: TagStats[]
}>()

const emit = defineEmits<{
  viewTag: [name: string]
}>()

const store = useStore()
const isDark = computed(() => store.colorScheme === 'dark')
const containerRef = ref<HTMLElement>()
const containerHeight = ref(400)

const COLOR_PALETTE_LIGHT = [
  '#2563eb', '#16a34a', '#9333ea', '#ea580c', '#0d9488',
  '#db2777', '#4f46e5', '#dc2626', '#0891b2', '#ca8a04',
]
const COLOR_PALETTE_DARK = [
  '#60a5fa', '#4ade80', '#c084fc', '#fb923c', '#2dd4bf',
  '#f472b6', '#818cf8', '#f87171', '#22d3ee', '#facc15',
]

const displayTags = computed(() => {
  return [...props.tags]
    .sort((a, b) => b.count - a.count)
    .slice(0, 100)
})

// Map for lookup in template title
const displayTagMap = computed(() => {
  const map: Record<string, TagStats> = {}
  for (const t of displayTags.value) {
    map[t.name] = t
  }
  return map
})

const minCount = computed(() => {
  if (displayTags.value.length === 0) return 0
  return displayTags.value[displayTags.value.length - 1].count
})
const maxCount = computed(() => {
  if (displayTags.value.length === 0) return 0
  return displayTags.value[0].count
})

function fontSize(count: number): number {
  if (maxCount.value === minCount.value) return 30
  const ratio = (count - minCount.value) / (maxCount.value - minCount.value)
  return Math.round(14 + ratio * 40)
}

function tagColor(index: number): string {
  const palette = isDark.value ? COLOR_PALETTE_DARK : COLOR_PALETTE_LIGHT
  return palette[index % palette.length]
}

interface LayoutWord {
  text: string
  size: number
  color: string
  x: number
  y: number
  rotate: number
  font: string
  weight?: string | number
  style?: string
  padding?: number
}

// Cloud layout state
const layoutWords = ref<LayoutWord[]>([])
const svgSize = reactive({ width: 0, height: 0 })

function computeSvgSize() {
  if (!containerRef.value) return
  const rect = containerRef.value.getBoundingClientRect()
  const w = Math.max(rect.width - 20, 300)
  const h = Math.min(Math.max(w * 0.5, 300), 500)
  svgSize.width = w
  svgSize.height = h
  containerHeight.value = h
}

function layoutCloud() {
  if (displayTags.value.length === 0) {
    layoutWords.value = []
    return
  }

  computeSvgSize()
  const { width, height } = svgSize
  if (width === 0 || height === 0) return

  const words = displayTags.value.map((t, i) => ({
    text: t.name,
    size: fontSize(t.count),
    color: tagColor(i),
  }))

  cloud()
    .size([width, height])
    .words(words)
    .padding(3)
    .rotate(() => (Math.random() > 0.5 ? 0 : 90))
    .font('system-ui, sans-serif')
    .spiral('archimedean')
    .on('end', (placed: cloud.Word[]) => {
      const colorByText = new Map(words.map(w => [w.text, w.color]))
      layoutWords.value = placed.map(w => ({
        ...w,
        text: w.text ?? '',
        color: colorByText.get(w.text ?? '') ?? '#888',
      })) as LayoutWord[]
    })
    .start()
}

watch(displayTags, () => {
  nextTick(() => layoutCloud())
}, { deep: true })

watch(isDark, () => {
  nextTick(() => layoutCloud())
})

onMounted(() => {
  const observer = new ResizeObserver(() => {
    layoutCloud()
  })
  if (containerRef.value) {
    observer.observe(containerRef.value)
  }
  onUnmounted(() => observer.disconnect())
  nextTick(() => layoutCloud())
})
</script>
