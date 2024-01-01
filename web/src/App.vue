<template>
  <div
    :class="{
      dark: colorScheme === 'dark',
    }"
  >
    <div class="min-h-screen transition-colors dark:bg-[#1a1a1a] dark:text-white">
      <Sidebar />
      <SidebarMask />
      <Navbar />
      <template v-if="!imagesFiltered.length">
        <Tip v-if="loading">
          <IconLoading class="mx-auto w-[60px] pb-2" :dark="colorScheme === 'light'" />
          <div class="text-center">
            数据加载中<br>
          </div>
        </Tip>
        <div v-if="!loading && notSettled" class="my-2 text-center">
          <template v-if="isTauri">
            <CButton class="block mx-auto mb-1" @click="setJsonPath">选择图片数据 JSON 磁盘路径</CButton>
            <CButton class="block mx-auto mb-1" @click="setImgDir">选择本地图片所在的文件夹路径</CButton>
            <div class="text-center">OR</div>
          </template>
          <template v-if="!userId">
            <div class="my-1 p-1" :title="isTauri ? '如果设置了用户 ID 的话则不读取本地图片数据' : ''">
              设置用户 ID
              <input
                v-model="userId"
                class="w-[250px] mx-1 rounded-md border px-1 py-0.5 leading-[22px] transition-colors hover:border-blue-500 dark:border-white/40 dark:bg-[#1a1a1a]"
                :placeholder="isTauri ? '设置 ID 的话则不读本地图片数据' : ''">
            </div>
            <CButton class="block mx-auto my-5 bg-[#409eff]" @click="saveReload">保存并刷新</CButton>
          </template>
        </div>
      </template>
      <template v-else>
        <MasonryView />
        <ImageViewer />
        <CButton class="block mx-auto my-5" @click="fetchUserBookmarks">{{ moreLoading ? '加载中' : '加载更多' }}</CButton>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { readTextFile } from '@tauri-apps/api/fs'
import { open as openFileDialog } from '@tauri-apps/api/dialog'

import { SettingType } from '@orilight/vue-settings'
import { useStore } from '@/store'

const store = useStore()

const {
  preferColorScheme,
  colorScheme,
  imagesFiltered,
  masonryConfig,
  filterConfig,
} = toRefs(store)

const loading = ref(true)
const moreLoading = ref(false)

const isTauri = !!(window as any).__TAURI__
const { __CONFIG__ } = window as any
const notSettled = !__CONFIG__.jsonPath || !__CONFIG__.userId
const userId = ref(__CONFIG__.userId)

async function setJsonPath() {
  const jsonPath = await openFileDialog()
  if (typeof jsonPath == 'string') {
    __CONFIG__.jsonPath = jsonPath
  }
}

async function setImgDir() {
  const imgDir = await openFileDialog({ directory: true })
  if (typeof imgDir == 'string') {
    __CONFIG__.imgDir = imgDir
  }
}

function saveReload() {
  if (isTauri) {
    localStorage.setItem('__PXCT_IMG_JSON_PATH', __CONFIG__.jsonPath)
    localStorage.setItem('__PXCT_IMG_DIR', __CONFIG__.imgDir)
  }
  localStorage.setItem('__PXCT_USER_ID', userId.value)
  location.reload()
}

onMounted(async () => {
  store.settings.register('preferColorScheme', preferColorScheme)
  store.settings.register('masonryConfig', masonryConfig, SettingType.Json, {
    deepMerge: true,
  })
  store.settings.register('restrictConfig', toRef(filterConfig.value, 'restrict'), SettingType.Json, {
    deepMerge: true,
  })

  try {
    if (__CONFIG__.userId) {
      await fetchUserBookmarks()
    } else if (isTauri && __CONFIG__.jsonPath) {
      const contents = await readTextFile(__CONFIG__.jsonPath)
      store.images = JSON.parse(contents)
    }
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
})

let maxBookmarkId = '0'
async function fetchUserBookmarks() {
  try {
    moreLoading.value = true
    const resp = await fetch(`https://hibiapi1.cocomi.eu.org/api/pixiv/favorite?id=${__CONFIG__.userId}&max_bookmark_id=${maxBookmarkId}`)
    const json = await resp.json()
    maxBookmarkId = new URL(json.next_url).searchParams.get('max_bookmark_id') || '0'
    store.images = store.images.concat(transformResData(json.illusts))
  } catch (error) {
    console.log('fetchUserBookmarks error: ', error)
  } finally {
    moreLoading.value = false
  }
}

function transformResData(data: any[]) {
  const results = []
  for (const json of data) {
    if (json.meta_single_page.original_image_url) {
      results.push({
        id: json.id,
        part: 0,
        len: 1,
        images: {
          s: json.image_urls.square_medium,
          m: json.image_urls.medium,
          l: json.image_urls.large,
          o: json.meta_single_page.original_image_url,
        },
        author: {
          id: json.user.id,
          name: json.user.name,
          account: json.user.account,
        },
        bookmark: json.total_bookmarks,
        created_at: json.create_date,
        ext: json.meta_single_page.original_image_url.split('.').pop(),
        sanity_level: json.sanity_level,
        size: [json.width, json.height],
        tags: json.tags,
        title: json.title,
        view: json.total_view,
        x_restrict: json.x_restrict,
        isAI: json.illust_ai_type === 2,
      })
    } else {
      results.push(...json.meta_pages.map((e: any, i: number) => ({
        id: json.id,
        part: i,
        len: json.meta_pages.length,
        images: {
          s: e.image_urls.square_medium,
          m: e.image_urls.medium,
          l: e.image_urls.large,
          o: e.image_urls.original,
        },
        author: {
          id: json.user.id,
          name: json.user.name,
          account: json.user.account,
        },
        bookmark: json.total_bookmarks,
        created_at: json.create_date,
        ext: e.image_urls.original.split('.').pop(),
        sanity_level: json.sanity_level,
        size: [json.width, json.height],
        tags: json.tags,
        title: json.title,
        view: json.total_view,
        x_restrict: json.x_restrict,
        isAI: json.illust_ai_type === 2,
      })))
    }
  }
  return results
}
</script>

<style>
body {
  overflow-y: scroll;
}
</style>
