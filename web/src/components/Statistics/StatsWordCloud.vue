<template>
  <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
    <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
      标签词云
    </h3>
    <div
      v-if="displayTags.length > 0"
      class="flex flex-wrap items-center justify-center gap-2 p-4"
    >
      <span
        v-for="(tag, index) in displayTags"
        :key="tag.name"
        :title="`${tag.translated_name || tag.name} (${tag.count})`"
        :style="{
          fontSize: `${fontSize(tag.count)}px`,
          color: tagColor(index),
        }"
        class="cursor-pointer transition-all duration-200 hover:scale-110 hover:opacity-80"
      >
        {{ tag.translated_name || tag.name }}
      </span>
    </div>
    <div
      v-else
      class="flex items-center justify-center py-12 text-sm text-gray-400 dark:text-gray-500"
    >
      暂无标签数据
    </div>
  </div>
</template>

<script setup lang="ts">
import { useStore } from '@/store'

interface TagStats {
  name: string
  translated_name: string | null
  count: number
}

const props = defineProps<{
  tags: TagStats[]
}>()

const store = useStore()
const isDark = computed(() => store.colorScheme === 'dark')

const COLOR_PALETTE_LIGHT = [
  '#2563eb', // blue-600
  '#16a34a', // green-600
  '#9333ea', // purple-600
  '#ea580c', // orange-600
  '#0d9488', // teal-600
  '#db2777', // pink-600
  '#4f46e5', // indigo-600
  '#dc2626', // red-600
  '#0891b2', // cyan-600
  '#ca8a04', // yellow-600
]

const COLOR_PALETTE_DARK = [
  '#60a5fa', // blue-400
  '#4ade80', // green-400
  '#c084fc', // purple-400
  '#fb923c', // orange-400
  '#2dd4bf', // teal-400
  '#f472b6', // pink-400
  '#818cf8', // indigo-400
  '#f87171', // red-400
  '#22d3ee', // cyan-400
  '#facc15', // yellow-400
]

const displayTags = computed(() => {
  return [...props.tags]
    .sort((a, b) => b.count - a.count)
    .slice(0, 100)
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
  return Math.round(12 + ratio * 36)
}

function tagColor(index: number): string {
  const palette = isDark.value ? COLOR_PALETTE_DARK : COLOR_PALETTE_LIGHT
  return palette[index % palette.length]
}
</script>
