<template>
  <div class="h-[60px]">
    <div
      class="fixed left-0 top-0 z-10 h-[60px] w-full border-b bg-white transition-all dark:border-white/20 dark:bg-[#242424]"
      :class="{
        'translate-y-[-70px]': (!showNav || imageViewer.show) && !showSidebar,
      }"
    >
      <div
        class="absolute left-0 top-0 flex"
        :class="{
          'w-full': filterConfig.search.enable,
        }"
        style="align-items: center"
      >
        <button
          class="h-[60px] w-[60px] hover:bg-gray-400/20"
          @click="showSidebar = !showSidebar"
        >
          <IconMenu class="mx-auto h-7 w-7" />
        </button>
        <button
          class="hidden h-[60px] w-[60px] hover:bg-gray-400/20 sm:block"
          @click="store.toggleSearch"
        >
          <IconSearch v-if="!filterConfig.search.enable" class="mx-auto h-6 w-6" />
          <IconClose v-else class="mx-auto h-6 w-6" />
        </button>
        <span v-if="!filterConfig.search.enable && (filterConfig.author.enable || filterConfig.tag.enable)" class="mr-1 hidden sm:block">
          {{ store.imagesFiltered.length }} Pcs
        </span>
        <button
          v-if="!filterConfig.search.enable && filterConfig.author.enable"
          class="m-0.5 hidden h-[20px] rounded-sm bg-blue-500/20 px-0.5 text-sm sm:block"
          @click="handleClickAuthor(filterConfig.author.id)"
        >
          {{ `作者：${filterConfig.author.id}` }}
        </button>
        <button
          v-if="!filterConfig.search.enable && filterConfig.tag.enable"
          class="m-0.5 hidden h-[20px] rounded-sm bg-black/20 px-0.5 text-sm sm:block"
          @click="handleClickTag(filterConfig.tag.name)"
        >
          {{ `标签：${filterConfig.tag.name}` }}
        </button>
        <div v-show="filterConfig.search.enable" class="mr-[60px] flex-1 sm:mr-0" style="position: relative;">
          <input
            ref="searchInputEl"
            v-model="searchInput"
            class="box-border h-[60px] w-full border-l border-gray-400/50 bg-transparent px-4 outline-none"
            type="text"
            placeholder="图片id/图片标题/作者id/作者昵称/标签，回车搜索"
            @keydown.enter="onSearchSubmit"
            @keydown.escape="showSuggestions = false"
            @keydown.down.prevent="onSuggestionKeydown(1)"
            @keydown.up.prevent="onSuggestionKeydown(-1)"
          >
          <div
            v-if="showSuggestions && suggestions.length > 0"
            class="absolute left-0 top-[60px] z-50 w-full rounded-b-lg border bg-white shadow-lg dark:border-white/20 dark:bg-[#242424]"
          >
            <button
              v-for="(tag, idx) in suggestions" :key="tag.name"
              class="flex w-full items-center px-4 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-white/10"
              :class="{ 'bg-gray-100 dark:bg-white/10': idx === highlightIdx }"
              @click="selectSuggestion(tag)"
              @mouseenter="highlightIdx = idx"
            >
              <span class="flex-1 truncate">{{ tag.translated_name || tag.name }}</span>
              <span class="ml-2 shrink-0 text-xs text-gray-400">{{ tag.count }} 幅</span>
            </button>
          </div>
        </div>
      </div>
      <button
        class="absolute right-0 block h-[60px] w-[60px] hover:bg-gray-400/20 sm:hidden"
        @click="store.toggleSearch"
      >
        <IconSearch v-if="!filterConfig.search.enable" class="mx-auto h-6 w-6" />
        <IconClose v-else class="mx-auto h-6 w-6" />
      </button>
      <div
        v-show="!filterConfig.search.enable"
        class="app-title mx-[60px] h-[60px] select-none text-center text-lg leading-[60px]"
        @click="navToTop"
      >
        <span class="hidden sm:inline-block">
          <span class="text-[#0398fa]">Pixiv</span>Collection
        </span>
        <svg class="inline-block cursor-pointer dark:fill-white" style="vertical-align: -0.5em;" viewBox="0 0 1024 1024" width="30" height="30"><path d="M698.8 337.6H325.2c-18.4 0-33.5-14.4-33.5-32s15.1-32 33.5-32h373.7c18.4 0 33.5 14.4 33.5 32-0.1 17.6-15.1 32-33.6 32z" fill="" p-id="4308"></path><path d="M508.4 547.8l1.8-1.8-1.8 1.8zM508.2 545.8l2.2 2.2c-0.7-0.8-1.4-1.5-2.2-2.2zM511.1 508.7l1.8 1.8-1.8-1.8z" fill="#FFFFFF" p-id="4309"></path><path d="M510.9 510.7l2.2-2.2c-0.8 0.7-1.5 1.4-2.2 2.2z" fill="#FFFFFF" p-id="4310"></path><path d="M544 472.4v246c0 17.6-14.4 32-32 32s-32-14.4-32-32v-246c0-17.6 14.4-32 32-32s32 14.4 32 32z" fill="" p-id="4311"></path><path d="M511.9 379c-8.3 0-15.8 3.1-21.5 8.3l-2.2 2.2-21.5 21.5L311 566.7c-12.4 12.4-12.4 32.8 0 45.3 12.4 12.4 32.8 12.4 45.3 0L512 456.2l155.8 155.7c12.4 12.4 32.8 12.4 45.3 0 12.4-12.4 12.4-32.8-0.1-45.2L557.3 411l-21.8-21.8-1.8-1.8c-5.7-5.3-13.4-8.5-21.8-8.4z" fill=""></path></svg>
      </div>
      <div class="absolute right-0 top-0 hidden lg:flex" style="align-items: center;">
        <CButton class="mr-2 h-[40px]" @click="reloadPage()">刷新</CButton>
        <CButton class="mr-2 h-[40px]" @click="emit('updatebookmark')">更新收藏</CButton>
        <button
          class="h-[60px] w-[60px] hover:bg-gray-400/20"
          @click="openGithub"
        >
          <IconGithub class="mx-auto h-6 w-6" />
        </button>
        <button
          class="h-[60px] w-[60px] hover:bg-gray-400/20"
          @click="store.toggleColorScheme"
        >
          <IconSun v-if="preferColorScheme === 'light'" class="mx-auto h-6 w-6" />
          <IconMoon v-if="preferColorScheme === 'dark'" class="mx-auto h-5 w-5" />
          <IconAuto v-if="preferColorScheme === 'auto'" class="mx-auto h-5 w-5" />
        </button>
        <!-- <button
          class="h-[60px] w-[60px] hover:bg-gray-400/20"
          @click="store.toggleFullscreen"
        >
          <IconShrink v-if="isFullscreen" class="mx-auto h-5 w-5" />
          <IconExpand v-else class="mx-auto h-5 w-5" />
        </button> -->
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useDebounceFn } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/tauri'
import { useStore } from '@/store'
import { LINK_GITHUB, NAVBAR_HIDE_DISTANCE } from '@/config'

const emit = defineEmits(['updatebookmark'])

const store = useStore()
const { preferColorScheme, showSidebar, showNav, imageViewer, isFullscreen, filterConfig } = toRefs(store)

// ---- Search autocomplete ----
const searchInput = ref('')
const showSuggestions = ref(false)
const suggestions = ref<{ name: string; translated_name: string | null; count: number }[]>([])
const highlightIdx = ref(-1)
const searchInputEl = ref<HTMLInputElement>()

const fetchSuggestions = useDebounceFn(async (value: string) => {
  if (!value.trim()) {
    suggestions.value = []
    showSuggestions.value = false
    return
  }
  try {
    const result = await invoke<any[]>('search_tags', { query: value.trim() })
    suggestions.value = result
    showSuggestions.value = result.length > 0
    highlightIdx.value = -1
  } catch (e) {
    console.error('search_tags failed:', e)
  }
}, 150)

watch(searchInput, val => {
  fetchSuggestions(val)
})

watch(() => filterConfig.value.search.enable, enable => {
  if (enable) {
    searchInput.value = ''
    nextTick(() => searchInputEl.value?.focus())
  } else {
    searchInput.value = ''
    showSuggestions.value = false
    suggestions.value = []
  }
})

function onSearchSubmit() {
  if (highlightIdx.value >= 0 && highlightIdx.value < suggestions.value.length) {
    selectSuggestion(suggestions.value[highlightIdx.value])
    return
  }
  showSuggestions.value = false
  store.updateSearchValue(searchInput.value)
}

function selectSuggestion(tag: { name: string; translated_name: string | null }) {
  searchInput.value = tag.translated_name || tag.name
  showSuggestions.value = false
  store.updateSearchValue(searchInput.value)
}

function onSuggestionKeydown(dir: number) {
  if (!showSuggestions.value || suggestions.value.length === 0) return
  highlightIdx.value = Math.min(Math.max(highlightIdx.value + dir, 0), suggestions.value.length - 1)
}

let oldY = 0

onMounted(() => {
  window.addEventListener('scroll', () => {
    const newY = document.documentElement.scrollTop
    if (newY > oldY && newY > NAVBAR_HIDE_DISTANCE) { showNav.value = false } else if (newY < oldY) { showNav.value = true }
    oldY = newY
  })
})

function navToTop() {
  document.documentElement.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

function openGithub() {
  window.open(LINK_GITHUB, '_blank')
}

function reloadPage() {
  location.reload()
}

function handleClickAuthor(authorId: number) {
  if (filterConfig.value.author.enable && filterConfig.value.author.id === authorId) {
    filterConfig.value.author.enable = false
    filterConfig.value.author.id = -1
  } else {
    filterConfig.value.author.id = authorId
    filterConfig.value.author.enable = true
  }
}

function handleClickTag(tagName: string) {
  if (filterConfig.value.tag.enable && filterConfig.value.tag.name === tagName) {
    filterConfig.value.tag.enable = false
    filterConfig.value.tag.name = ''
  } else {
    filterConfig.value.tag.name = tagName
    filterConfig.value.tag.enable = true
  }
}
</script>

<style>
@media screen and (max-width: 1024px) {
  .app-title {
    display: flex;
    justify-content: end;
    align-items: center;
  }
}
@media screen and (max-width: 640px) {
  .app-title {
    display: block;
  }
}
</style>
