<template>
  <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
    <h3 class="mb-5 text-sm font-bold text-gray-700 dark:text-gray-200">
      作者发现时间线
    </h3>

    <!-- Empty state -->
    <div
      v-if="groups.length === 0"
      class="flex items-center justify-center py-16 text-gray-400 dark:text-gray-500"
    >
      <span class="text-sm">暂无数据</span>
    </div>

    <!-- Timeline -->
    <div
      v-else
      class="relative ml-3"
    >
      <!-- Vertical line -->
      <div class="absolute left-[7px] top-2 h-[calc(100%-12px)] w-0.5 bg-gray-200 dark:bg-white/10"></div>

      <div
        v-for="group in groups"
        :key="group.year"
        class="relative"
      >
        <!-- Year header (sticky while scrolling within section) -->
        <div
          class="sticky top-0 z-10 flex cursor-pointer items-center gap-3 py-3 backdrop-blur-sm"
          :class="isDark ? 'bg-[#242424]/90' : 'bg-white/90'"
          role="button"
          :tabindex="0"
          @click="toggleYear(group.year)"
          @keydown.enter="toggleYear(group.year)"
          @keydown.space.prevent="toggleYear(group.year)"
        >
          <!-- Timeline dot -->
          <div class="relative z-10 flex h-[15px] w-[15px] shrink-0 items-center justify-center">
            <div
              class="h-[15px] w-[15px] rounded-full border-2 border-blue-400 bg-white transition-colors dark:border-blue-500 dark:bg-[#242424]"
            ></div>
            <div class="absolute inset-[3px] rounded-full bg-blue-400 dark:bg-blue-500"></div>
          </div>

          <!-- Year label -->
          <span class="select-none text-base font-bold tracking-tight text-gray-800 dark:text-gray-100">
            {{ group.year }}
          </span>

          <!-- Author count chip -->
          <span class="select-none rounded-md bg-gray-100 px-2 py-0.5 text-xs font-medium text-gray-500 dark:bg-white/10 dark:text-gray-400">
            {{ group.authors.length }} 位
          </span>

          <!-- Chevron -->
          <svg
            class="ml-auto h-4 w-4 text-gray-400 transition-transform duration-200 dark:text-gray-500"
            :class="{ 'rotate-180': !collapsed[group.year] }"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="2"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M19.5 8.25l-7.5 7.5-7.5-7.5"
            ></path>
          </svg>
        </div>

        <!-- Author list -->
        <div
          v-show="!collapsed[group.year]"
          class="ml-[46px] space-y-0.5 pb-4"
        >
          <div
            v-for="author in group.authors"
            :key="author.author_id"
            class="group flex cursor-pointer items-center justify-between rounded-lg px-3 py-2.5 transition-colors hover:bg-blue-50/60 dark:hover:bg-blue-900/10"
            @click="emit('viewAuthor', author.author_id)"
          >
            <div class="flex min-w-0 items-center gap-2">
              <span class="truncate text-sm font-medium text-blue-600 transition-colors group-hover:text-blue-800 dark:text-blue-400 dark:group-hover:text-blue-300">
                {{ author.author_name || '(佚名)' }}
              </span>
              <span class="shrink-0 text-xs text-gray-400 dark:text-gray-500">
                @{{ author.author_account }}
              </span>
            </div>
            <span class="ml-2 shrink-0 whitespace-nowrap text-xs text-gray-500 dark:text-gray-400">
              作品数 {{ author.works_count }} 件
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useStore } from '@/store'

interface AuthorDiscovery {
  author_id: number
  author_name: string
  author_account: string
  first_year: number
  works_count: number
}

interface YearGroup {
  year: number
  authors: AuthorDiscovery[]
}

const props = defineProps<{
  authorDiscovery: AuthorDiscovery[]
}>()

const emit = defineEmits<{
  viewAuthor: [id: number]
}>()

const store = useStore()

const isDark = computed(() => store.colorScheme === 'dark')

// Group by first_year, sorted DESC
const groups = computed<YearGroup[]>(() => {
  const map = new Map<number, AuthorDiscovery[]>()
  for (const author of props.authorDiscovery) {
    const list = map.get(author.first_year)
    if (list) {
      list.push(author)
    } else {
      map.set(author.first_year, [author])
    }
  }
  return Array.from(map.entries())
    .map(([year, authors]) => ({ year, authors: authors.slice(0, 100) }))
    .sort((a, b) => b.year - a.year)
})

// Collapsible state
const collapsed = reactive<Record<number, boolean>>({})

function toggleYear(year: number): void {
  collapsed[year] = !collapsed[year]
}

</script>
