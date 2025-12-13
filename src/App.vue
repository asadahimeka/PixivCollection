<template>
  <div :class="{ dark: colorScheme === 'dark' }">
    <div class="min-h-screen transition-colors dark:bg-[#1a1a1a] dark:text-white">
      <Sidebar @updatebookmark="updateBookmark()" />
      <SidebarMask />
      <Navbar @updatebookmark="updateBookmark()" />
      <template v-if="!store.imagesFiltered.length">
        <Tip v-if="loading">
          <!-- <IconLoading class="mx-auto w-[60px] pb-2" :dark="colorScheme === 'light'" /> -->
          <div class="text-center">
            数据加载中<br>
          </div>
        </Tip>
        <Tip v-if="!loading && !notSettled">
          <div class="text-center">
            暂无数据<br>请更新收藏
          </div>
        </Tip>
        <div v-if="!loading && notSettled" class="my-2 text-center">
          <div class="my-1 p-1">
            设置用户 ID
            <input
              v-model="userId"
              class="mx-1 w-[250px] rounded-md border px-1 py-0.5 leading-[22px] transition-colors hover:border-blue-500 dark:border-white/40 dark:bg-[#1a1a1a]"
              placeholder="输入你的用户 ID (数字)"
            >
          </div>
          <div class="text-center">
            <p>如何获取你的 ID：</p>
            <p class="mb-2">访问你的个人主页然后复制地址栏中的数字</p>
            <img
              style="margin: auto;"
              src="https://upload-bbs.miyoushe.com/upload/2024/01/28/190122060/7c26a8882d5f9788e3ff224bffe484ce_8510401254923221347.png"
              alt=""
            >
          </div>
          <p class="my-2">设置成功后请点击右上角“更新收藏”</p>
          <CButton class="mx-auto my-5 block bg-[#409eff]" @click="saveReload">保存并刷新</CButton>
        </div>
      </template>
      <template v-else>
        <MasonryView />
        <ImageViewer />
        <CButton v-if="store.masonryConfig.sliceLocalImages && !store.loadEnd" class="mx-auto my-5 block" @click="fetchMore">{{ moreLoading ? '加载中' : '加载更多' }}</CButton>
      </template>
      <Transition name="fade">
        <div v-if="showModalMsg" class="bookmark-update-msg">
          <pre ref="modalMsgEl" class="bum-cnt" v-html="modalMsg"></pre>
          <i class="bum-close" @click="closeMsgModal()">×</i>
        </div>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { SettingType } from '@orilight/vue-settings'
import { useDebounceFn } from '@vueuse/core'
import localforage from 'localforage'
import { useStore } from '@/store'
import { sleep, transformResData } from '@/utils'

const store = useStore()

const {
  preferColorScheme,
  colorScheme,
  masonryConfig,
  filterConfig,
} = toRefs(store)

const loading = ref(true)
const moreLoading = ref(false)

const w = (window as any)
const { __CONFIG__ } = w
const notSettled = !__CONFIG__.userId
const userId = ref(__CONFIG__.userId)

const modalMsgEl = ref<HTMLElement>()
const showModalMsg = ref(false)
const modalMsg = ref('')
watch(modalMsg, () => {
  nextTick(() => {
    modalMsgEl.value?.scrollTo({ top: modalMsgEl.value.scrollHeight })
  })
})

function closeMsgModal() {
  // showModalMsg.value = false
  // modalMsg.value = ''
  location.reload()
}

async function saveReload() {
  localStorage.setItem('__PXCT_USER_ID', userId.value)
  location.reload()
}

onMounted(async () => {
  store.settings.register('preferColorScheme', preferColorScheme as any)
  store.settings.register('masonryConfig', masonryConfig as any, SettingType.Json, {
    deepMerge: true,
  })
  store.settings.register('restrictConfig', toRef(filterConfig.value, 'restrict') as any, SettingType.Json, {
    deepMerge: true,
  })

  await init()
})

onUnmounted(() => {
  store.settings.unregisterAll()
})

const isInit = ref(false)
watch(
  () => store.filterConfig,
  useDebounceFn(() => {
    if (!isInit.value) return
    showModalMsg.value = true
    modalMsg.value = '<p style="text-align:center">加载中</p>'
    setTimeout(() => {
      console.time('filter')
      if (store.masonryConfig.sliceLocalImages) {
        store.curPageCursor = 0
        store.loadEnd = false
        store.imagesFiltered = []
        store.loadImagesByPage()
      } else {
        store.loadFilteredImages()
      }
      console.timeEnd('filter')
      document.documentElement.scrollTop = 0
      showModalMsg.value = false
      modalMsg.value = ''
    }, 100)
  }, 200),
  { deep: true },
)

async function init() {
  try {
    if (!__CONFIG__.userId) return
    loading.value = true
    store.curPageCursor = 0
    store.loadEnd = false
    store.imagesFiltered = []
    console.time('db')
    const contents: any[] = (await localforage.getItem(`__PXCT_BOOKMARKS_u${__CONFIG__.userId}`)) || []
    console.timeEnd('db')
    console.time('init')
    w.__fullImages__ = contents
    store.updateFullCounts()
    if (store.masonryConfig.sliceLocalImages) {
      store.curPageCursor = 0
      store.loadImagesByPage(true)
    } else {
      store.imagesFiltered = contents
    }
    console.timeEnd('init')
    console.time('nextTick')
    nextTick(() => {
      console.timeEnd('nextTick')
    })
  } catch (e) {
    console.error(e)
    const msg = (e as Error).message || JSON.stringify(e)
    showModalMsg.value = true
    modalMsg.value += `<br><div style="color:#ff6565">${msg}</div>`
  } finally {
    loading.value = false
    setTimeout(() => {
      isInit.value = true
    }, 500)
  }
}

async function fetchMore() {
  if (moreLoading.value) return
  moreLoading.value = true
  if (__CONFIG__.userId && store.masonryConfig.sliceLocalImages) {
    store.loadImagesByPage()
  } else {
    store.loadEnd = true
  }
  moreLoading.value = false
}

let maxBookmarkId = '0'
async function updateBookmark() {
  try {
    if (!__CONFIG__.userId) return
    moreLoading.value = true
    store.loadEnd = false

    showModalMsg.value = true
    modalMsg.value = '开始更新收藏...<br>'

    const lastIdKey = `__PXCT_LAST_PID_u${__CONFIG__.userId}`
    const bookmarkKey = `__PXCT_BOOKMARKS_u${__CONFIG__.userId}`
    const lastId = (await localforage.getItem(lastIdKey)) || ''
    const bookmarkCache: any[] = (await localforage.getItem(bookmarkKey)) || []

    const illusts: any[] = []
    let stop = false
    while (!store.loadEnd && !stop) {
      const res = await fetchUserBookmarks()
      for (const it of res) {
        if (it.id == lastId) {
          stop = true
          break
        }
        illusts.push(it)
      }

      modalMsg.value += `更新中：${illusts.length}<br>等待1秒...<br>`
      await sleep(1000)
    }

    console.log('illusts: ', illusts)
    modalMsg.value += '更新完成<br>'
    maxBookmarkId = '0'
    if (illusts.length) {
      modalMsg.value += '存储中...<br>'
      await localforage.setItem(lastIdKey, illusts[0].id)
      await localforage.setItem(bookmarkKey, illusts.concat(bookmarkCache))
      modalMsg.value += '存储完成<br>'
    } else {
      modalMsg.value += '暂无更新<br>'
    }
  } catch (error) {
    console.log('fetchUserBookmarks: ', error)
    showModalMsg.value = true
    modalMsg.value += `<br><div style="color:#ff6565">更新出错: ${error}</div>`
  } finally {
    moreLoading.value = false
  }
}

async function fetchUserBookmarks() {
  // const url = `https://hibiapi.cocomi.eu.org/api/pixiv/favorite?id=${__CONFIG__.userId}&max_bookmark_id=${maxBookmarkId}&_t=${Date.now()}`
  // console.log('url: ', url)
  // const resp = await fetch(url)
  // const { next_url, illusts = [] } = await resp.json()
  const { next_url, illusts = [] } = await (parent as any).__pxcl.fetchUserBookmarks(maxBookmarkId)
  if (!next_url || !illusts.length) {
    store.loadEnd = true
    maxBookmarkId = '0'
  } else {
    maxBookmarkId = new URL(next_url).searchParams.get('max_bookmark_id') || '0'
  }
  const res = transformResData(illusts)
  return res
}
</script>

<style>
body {
  overflow-y: scroll;
}

body:has(.bookmark-update-msg) {
  overflow-y: hidden;
}

.bookmark-update-msg {
  position: fixed;
  z-index: 200;
  top: 0;
  left: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.9);
  overflow-y: auto;
}

.bookmark-update-msg .bum-cnt {
  min-width: 500px;
  min-width: min(500px, 98vw);
  max-width: 98vw;
  max-height: 100vh;
  padding: 20px;
  color: white;
  white-space: pre-wrap;
  font-family: Consolas, 'Courier New', Courier, monospace;
  font-size: 18px;
  overflow-y: auto;
}

.bookmark-update-msg .bum-close {
  position: absolute;
  top: 16px;
  right: 36px;
  font-size: 36px;
  padding: 10px;
  color: white;
  font-style: normal;
  font-family: SimSun, monospace;
  font-weight: bold;
  cursor: pointer;
}
</style>
