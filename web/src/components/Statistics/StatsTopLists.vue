<template>
  <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
    <!-- Top Bookmarked -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
        收藏数最高 TOP10
      </h3>
      <div class="overflow-hidden rounded-lg border dark:border-white/10">
        <table class="w-full text-sm">
          <thead>
            <tr class="bg-gray-50 text-gray-600 dark:bg-white/5 dark:text-gray-400">
              <th class="w-12 px-3 py-2 text-center font-medium">
                排名
              </th>
              <th class="px-3 py-2 text-left font-medium">
                作品
              </th>
              <th class="px-3 py-2 text-left font-medium">
                作者
              </th>
              <th class="w-24 px-3 py-2 text-right font-medium">
                收藏数
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(item, index) in topBookmarked"
              :key="item.id"
              :class="[
                rankRowBg(index),
                index % 2 === 0 && index >= 3 ? 'bg-white dark:bg-transparent' : '',
                index % 2 === 1 && index >= 3 ? 'bg-gray-50/50 dark:bg-white/[0.03]' : '',
              ]"
              class="border-t border-gray-100 transition-colors hover:bg-blue-50/50 dark:border-white/5 dark:hover:bg-blue-900/10"
            >
              <td class="px-3 py-2 text-center">
                <span
                  class="inline-flex h-6 w-6 items-center justify-center rounded-full text-xs font-bold" :class="[
                    rankBadgeClass(index),
                  ]"
                >
                  {{ index + 1 }}
                </span>
              </td>
              <td class="max-w-0 px-3 py-2">
                <a
                  :href="artworkLink(item.id)"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="block truncate text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                  :title="item.title"
                  @click.prevent="openArtwork(item.id)"
                >
                  {{ item.title }}
                </a>
              </td>
              <td class="px-3 py-2 text-gray-600 dark:text-gray-400">
                {{ item.author_name }}
              </td>
              <td class="px-3 py-2 text-right font-medium tabular-nums text-gray-900 dark:text-gray-100">
                {{ formatNumber(item.value) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Top Viewed -->
    <div class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
      <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
        浏览数最高 TOP10
      </h3>
      <div class="overflow-hidden rounded-lg border dark:border-white/10">
        <table class="w-full text-sm">
          <thead>
            <tr class="bg-gray-50 text-gray-600 dark:bg-white/5 dark:text-gray-400">
              <th class="w-12 px-3 py-2 text-center font-medium">
                排名
              </th>
              <th class="px-3 py-2 text-left font-medium">
                作品
              </th>
              <th class="px-3 py-2 text-left font-medium">
                作者
              </th>
              <th class="w-24 px-3 py-2 text-right font-medium">
                浏览数
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(item, index) in topViewed"
              :key="item.id"
              :class="[
                rankRowBg(index),
                index % 2 === 0 && index >= 3 ? 'bg-white dark:bg-transparent' : '',
                index % 2 === 1 && index >= 3 ? 'bg-gray-50/50 dark:bg-white/[0.03]' : '',
              ]"
              class="border-t border-gray-100 transition-colors hover:bg-blue-50/50 dark:border-white/5 dark:hover:bg-blue-900/10"
            >
              <td class="px-3 py-2 text-center">
                <span
                  class="inline-flex h-6 w-6 items-center justify-center rounded-full text-xs font-bold" :class="[
                    rankBadgeClass(index),
                  ]"
                >
                  {{ index + 1 }}
                </span>
              </td>
              <td class="max-w-0 px-3 py-2">
                <a
                  :href="artworkLink(item.id)"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="block truncate text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                  :title="item.title"
                  @click.prevent="openArtwork(item.id)"
                >
                  {{ item.title }}
                </a>
              </td>
              <td class="px-3 py-2 text-gray-600 dark:text-gray-400">
                {{ item.author_name }}
              </td>
              <td class="px-3 py-2 text-right font-medium tabular-nums text-gray-900 dark:text-gray-100">
                {{ formatNumber(item.value) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>

  <!-- Hidden Gems -->
  <div v-if="hiddenGems && hiddenGems.length > 0" class="rounded-xl bg-white p-5 shadow-sm dark:bg-[#242424]">
    <h3 class="mb-4 text-sm font-bold text-gray-700 dark:text-gray-200">
      收藏效率 TOP10
    </h3>
    <div class="overflow-hidden rounded-lg border dark:border-white/10">
      <table class="w-full text-sm">
        <thead>
          <tr class="bg-gray-50 text-gray-600 dark:bg-white/5 dark:text-gray-400">
            <th class="w-12 px-3 py-2 text-center font-medium">
              排名
            </th>
            <th class="px-3 py-2 text-left font-medium">
              标题
            </th>
            <th class="px-3 py-2 text-left font-medium">
              作者
            </th>
            <th class="w-20 px-3 py-2 text-right font-medium">
              收藏
            </th>
            <th class="w-20 px-3 py-2 text-right font-medium">
              浏览
            </th>
            <th class="w-20 px-3 py-2 text-right font-medium">
              效率
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(item, index) in hiddenGems"
            :key="item.id"
            :class="[
              rankRowBg(index),
              index % 2 === 0 && index >= 3 ? 'bg-white dark:bg-transparent' : '',
              index % 2 === 1 && index >= 3 ? 'bg-gray-50/50 dark:bg-white/[0.03]' : '',
            ]"
            class="border-t border-gray-100 transition-colors hover:bg-blue-50/50 dark:border-white/5 dark:hover:bg-blue-900/10"
          >
            <td class="px-3 py-2 text-center">
              <span
                class="inline-flex h-6 w-6 items-center justify-center rounded-full text-xs font-bold" :class="[
                  rankBadgeClass(index),
                ]"
              >
                {{ index + 1 }}
              </span>
            </td>
            <td class="max-w-0 px-3 py-2">
              <a
                :href="artworkLink(item.id)"
                target="_blank"
                rel="noopener noreferrer"
                class="block truncate text-blue-600 transition-colors hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                :title="item.title"
                @click.prevent="openArtwork(item.id)"
              >
                {{ item.title }}
              </a>
            </td>
            <td class="px-3 py-2 text-gray-600 dark:text-gray-400">
              {{ item.author_name }}
            </td>
            <td class="px-3 py-2 text-right tabular-nums text-gray-900 dark:text-gray-100">
              {{ formatNumber(item.bookmark) }}
            </td>
            <td class="px-3 py-2 text-right tabular-nums text-gray-600 dark:text-gray-400">
              {{ formatNumber(item.view) }}
            </td>
            <td class="px-3 py-2 text-right font-medium tabular-nums text-gray-900 dark:text-gray-100">
              {{ item.ratio.toFixed(2) }}%
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { LINK_PIXIV_ARTWORK } from '@/config'

interface TopWork {
  id: number
  title: string
  value: number
  author_name: string
}

interface HiddenGem {
  id: number
  title: string
  author_name: string
  bookmark: number
  view: number
  ratio: number
}

const props = defineProps<{
  topBookmarked: TopWork[]
  topViewed: TopWork[]
  hiddenGems?: HiddenGem[]
}>()

function rankRowBg(index: number): string {
  if (index === 0) return 'bg-amber-50 dark:bg-amber-900/20'
  if (index === 1) return 'bg-slate-50 dark:bg-slate-700/20'
  if (index === 2) return 'bg-orange-50 dark:bg-orange-900/20'
  return ''
}

function rankBadgeClass(index: number): string {
  if (index === 0) return 'bg-amber-400 text-white dark:bg-amber-500'
  if (index === 1) return 'bg-slate-400 text-white dark:bg-slate-500'
  if (index === 2) return 'bg-orange-400 text-white dark:bg-orange-500'
  return 'bg-gray-200 text-gray-600 dark:bg-gray-600 dark:text-gray-300'
}

function artworkLink(id: number): string {
  return LINK_PIXIV_ARTWORK.replace('{id}', String(id))
}

function openArtwork(id: number): void {
  window.open(artworkLink(id), '_blank')
}

function formatNumber(n: number): string {
  return n.toLocaleString()
}
</script>
