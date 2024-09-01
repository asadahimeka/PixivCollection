<template>
  <div :class="{ dark: colorScheme === 'dark' }">
    <div class="min-h-screen transition-colors dark:bg-[#1a1a1a] dark:text-white">
      <Sidebar />
      <SidebarMask />
      <Navbar @updatebookmark="updateBookmark()" />
      <template v-if="!imagesFiltered.length">
        <Tip v-if="loading">
          <IconLoading class="mx-auto w-[60px] pb-2" :dark="colorScheme === 'light'" />
          <div class="text-center">
            数据加载中<br>
          </div>
        </Tip>
        <Tip v-if="!loading && !notSettled">
          <div class="text-center">
            暂无数据<br>
          </div>
        </Tip>
        <div v-if="!loading && notSettled" class="my-2 text-center">
          <template v-if="isTauri">
            <div class="my-1 p-1">
              设置 Pixiv RefreshToken
              <input
                v-model="pxderToken"
                class="mx-1 w-[330px] rounded-md border px-1 py-0.5 leading-[22px] transition-colors hover:border-blue-500 dark:border-white/40 dark:bg-[#1a1a1a]"
                placeholder="输入 Pixiv RefreshToken 以获取收藏夹数据"
              >
            </div>
            <div class="my-1 p-1">
              设置 HTTP 代理
              <input
                v-model="pxderProxy"
                class="mx-1 w-[440px] rounded-md border px-1 py-0.5 leading-[22px] transition-colors hover:border-blue-500 dark:border-white/40 dark:bg-[#1a1a1a]"
                placeholder="如果无法直接访问 Pixiv 请设置, 如: http://127.0.0.1:7890"
              >
            </div>
            <div class="my-1 p-1" style="display: flex;justify-content: center;align-items: center;flex-wrap: wrap;">
              设置存储路径
              <CButton class="mx-2" @click="setImgDir">选择图片与数据的存储路径</CButton>
              <p v-if="displayImgDir">{{ displayImgDir }}</p>
            </div>
            <p class="my-2">设置成功后请点击右上角“更新收藏”</p>
            <CButton class="mx-auto my-5 block bg-[#409eff]" @click="saveReload">保存并刷新</CButton>
            <hr>
            <div class="my-2 text-center font-bold">OR</div>
          </template>
          <template v-if="!__CONFIG__.userId">
            <div class="my-1 p-1" :title="isTauri ? '如果设置了用户 ID 的话则不读取本地图片数据' : ''">
              设置用户 ID
              <input
                v-model="userId"
                class="mx-1 w-[250px] rounded-md border px-1 py-0.5 leading-[22px] transition-colors hover:border-blue-500 dark:border-white/40 dark:bg-[#1a1a1a]"
                :placeholder="isTauri ? '设置 ID 的话则不读本地图片数据' : '输入你的用户 ID (数字)'"
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
            <CButton class="mx-auto my-5 block bg-[#409eff]" @click="saveReload">保存并刷新</CButton>
          </template>
        </div>
      </template>
      <template v-else>
        <MasonryView />
        <ImageViewer />
        <CButton v-if="!store.loadEnd" class="mx-auto my-5 block" @click="fetchMore">{{ moreLoading ? '加载中' : '加载更多' }}</CButton>
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
import { readTextFile, writeTextFile } from '@tauri-apps/api/fs'
import { open as openFileDialog } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import { join } from '@tauri-apps/api/path'
import { Command } from '@tauri-apps/api/shell'

import { SettingType } from '@orilight/vue-settings'
import { useDebounceFn } from '@vueuse/core'
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

const w = (window as any)
const isTauri = !!w.__TAURI__
const { __CONFIG__ } = w
const notSettled = !(__CONFIG__.imgDir && __CONFIG__.pxderToken) && !__CONFIG__.userId
const userId = ref(__CONFIG__.userId)
const pxderToken = ref(__CONFIG__.pxderToken)
const pxderProxy = ref(__CONFIG__.pxderProxy)

const displayImgDir = ref(__CONFIG__.imgDir || '')
async function setImgDir() {
  const imgDir = await openFileDialog({ directory: true })
  if (typeof imgDir == 'string') {
    __CONFIG__.imgDir = imgDir
    __CONFIG__.jsonPath = await join(imgDir, 'data', 'images.json')
    displayImgDir.value = imgDir
  }
}

const modalMsgEl = ref<HTMLElement>()
const showModalMsg = ref(false)
const modalMsg = ref('')
watch(modalMsg, () => {
  nextTick(() => {
    modalMsgEl.value?.scrollTo({ top: modalMsgEl.value.scrollHeight })
  })
})

let startUpdateCp: any = null
const killUpdateCp = () => {
  try { startUpdateCp?.kill() } catch (err) {}
}
onUnmounted(killUpdateCp)
async function updateBookmark() {
  showModalMsg.value = true
  modalMsg.value = 'Start updating bookmark...\n'

  const exeDir = await invoke<string>('get_executable_dir')
  console.log('cwd: ', exeDir)
  modalMsg.value += `Current executable dir: ${exeDir}\n`

  const cmdCwd = await join(exeDir, import.meta.env.DEV ? '../../../' : '.', 'pxder')
  const cmdPath = await join(cmdCwd, 'start.bat')
  console.log('cmdPath: ', cmdPath)
  modalMsg.value += `Execute script: ${cmdPath}\n`

  // 创建命令
  const startCmd = new Command('cmd', ['/C', cmdPath], { cwd: cmdCwd })
  startCmd.on('close', data => {
    const msg = `UpdateBookmark command finished with code ${data.code} and signal ${data.signal}.`
    console.log(msg)
    modalMsg.value += msg
  })
  startCmd.on('error', error => {
    const msg = `UpdateBookmark command error: "${error}".`
    console.error(msg)
    modalMsg.value += `<br><div style="color:#ff6565">${msg}</div>`
  })
  // 监听 stdout 实时输出
  startCmd.stdout.on('data', (line: string) => {
    line = line.replaceAll('\x1B[2K\x1B[1G', '')
    console.log('STDOUT: ', line)
    // 在这里处理实时输出，例如更新UI
    const str = line.split('__EOS__').filter(Boolean).map(e => {
      const m = e.match(/(.*)\$\${%c\<(.*)\>%(.*)}\$\$(.*)/)
      return m ? `<span>${m[1] || ''}<span style="${m[2] || ''}">${m[3] || ''}</span>${m[4] || ''}</span>` : e
    }).join('')
    modalMsg.value += str ? `<div>${str}</div>` : (line || '<br>')
  })
  // 监听 stderr 实时输出
  startCmd.stderr.on('data', (line: string) => {
    console.error('STDERR: ', line)
    // 在这里处理错误输出
    const str = line.split('__EOS__').filter(Boolean).map(e => {
      const m = e.match(/(.*)\$\${(%c)?\<(.*)\>%(.*)}\$\$(.*)/)
      return m ? `<span>${m[1] || ''}<span style="${m[3]?.replaceAll('red', '#ff6565') || ''}">${m[4] || ''}</span>${m[5] || ''}</span>` : e
    }).join('')
    modalMsg.value += str ? `<div style="color:#ff6565">${str}</div>` : (line ? `<div style="color:#ff6565">${line}</span>` : '<br>')
  })
  // 执行命令
  startUpdateCp = await startCmd.spawn()
}

function closeMsgModal() {
  showModalMsg.value = false
  modalMsg.value = ''
  killUpdateCp()
}

async function saveReload() {
  if (isTauri) {
    if (!/^[\w-]{43}$/.test(pxderToken.value)) {
      alert('请输入有效的 RefreshToken')
      return
    }
    if (!/^http\:\/\/\d+\.\d+\.\d+\.\d+\:\d+$/.test(pxderProxy.value)) {
      alert('请输入有效的 HTTP 代理')
      return
    }

    localStorage.setItem('__PXCT_IMG_JSON_PATH', __CONFIG__.jsonPath)
    localStorage.setItem('__PXCT_IMG_DIR', __CONFIG__.imgDir)
    localStorage.setItem('__PXCT_PXDER_TOKEN', pxderToken.value)
    localStorage.setItem('__PXCT_PXDER_PROXY', pxderProxy.value)

    const cwd = await invoke<string>('get_executable_dir')
    const pxderPath = await join(cwd, import.meta.env.DEV ? '../../../' : '.', 'pxder')
    const configPath = await join(pxderPath, 'src/config/config.json')
    const configJson = JSON.parse(await readTextFile(configPath).catch(() => '{"download":{}}'))
    configJson.download.path = __CONFIG__.imgDir
    configJson.refresh_token = pxderToken.value
    configJson.proxy = pxderProxy.value
    await writeTextFile(configPath, JSON.stringify(configJson))
    await writeTextFile(await join(pxderPath, 'data/_last_id.json'), '{"id":""}').catch(() => {})
    await writeTextFile(await join(pxderPath, 'data/_last_illusts.json'), '[]').catch(() => {})
  }
  localStorage.setItem('__PXCT_USER_ID', userId.value)
  await invoke('restart_app')
}

onMounted(async () => {
  store.settings.register('preferColorScheme', preferColorScheme)
  store.settings.register('masonryConfig', masonryConfig, SettingType.Json, {
    deepMerge: true,
  })
  store.settings.register('restrictConfig', toRef(filterConfig.value, 'restrict'), SettingType.Json, {
    deepMerge: true,
  })

  await init()
})

onUnmounted(() => {
  store.settings.unregisterAll()
})

watch(
  () => store.filterConfig,
  useDebounceFn(() => {
    if (store.masonryConfig.sliceLocalImages) {
      store.curPageCursor = 0
      store.loadEnd = false
      store.imagesFiltered = []
      store.loadImagesByPage()
    } else {
      store.loadFilteredImages()
    }
    document.documentElement.scrollTop = 0
  }, 250),
  { deep: true },
)

async function init() {
  try {
    loading.value = true
    store.curPageCursor = 0
    store.loadEnd = false
    store.imagesFiltered = []
    if (__CONFIG__.userId) {
      await fetchUserBookmarks()
    } else if (isTauri && __CONFIG__.jsonPath && __CONFIG__.imgDir) {
      if (store.masonryConfig.loadImagesJsonByLocalHttp || store.masonryConfig.loadImageByLocalHttp) {
        if (!sessionStorage.getItem('local_server_started')) {
          await invoke('start_local_server', { base: __CONFIG__.imgDir })
          sessionStorage.setItem('local_server_started', 'true')
        }
      }

      let contents = []
      if (store.masonryConfig.loadImagesJsonByLocalHttp) {
        contents = await fetch(`http://localhost:32154/data/images.json?base=${__CONFIG__.imgDir}`).then(r => r.json())
      } else if (__CONFIG__.jsonPath) {
        contents = JSON.parse(await readTextFile(__CONFIG__.jsonPath))
      }

      w.__fullImages__ = contents
      store.updateFullCounts()

      if (store.masonryConfig.sliceLocalImages) {
        store.curPageCursor = 0
        store.loadImagesByPage(true)
      } else {
        store.imagesFiltered = contents
      }
    }
  } catch (e) {
    console.error(e)
    showModalMsg.value = true
    modalMsg.value = (e as Error).message || JSON.stringify(e)
  } finally {
    loading.value = false
  }
}

async function fetchMore() {
  if (moreLoading.value) return
  moreLoading.value = true
  if (__CONFIG__.userId) {
    await fetchUserBookmarks()
  } else if (isTauri && __CONFIG__.jsonPath && store.masonryConfig.sliceLocalImages) {
    store.loadImagesByPage()
  } else {
    store.loadEnd = true
  }
  moreLoading.value = false
}

let maxBookmarkId = '0'
async function fetchUserBookmarks() {
  try {
    moreLoading.value = true
    store.loadEnd = false
    const resp = await fetch(`https://hibiapi.cocomi.eu.org/api/pixiv/favorite?id=${__CONFIG__.userId}&max_bookmark_id=${maxBookmarkId}&_t=${Date.now().toString().slice(0, 8)}`)
    const json = await resp.json()
    maxBookmarkId = new URL(json.next_url).searchParams.get('max_bookmark_id') || '0'
    const res = transformResData(json.illusts)
    if (!res.length) {
      store.loadEnd = true
      return
    }
    w.__fullImages__ = w.__fullImages__.concat(res)
    store.updateFullCounts()
    store.loadFilteredImages()
  } catch (error) {
    console.log('fetchUserBookmarks error: ', error)
    showModalMsg.value = true
    modalMsg.value = `fetchUserBookmarks error: ${error}`
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
  font-family: monospace;
  font-weight: bold;
  cursor: pointer;
}
</style>
