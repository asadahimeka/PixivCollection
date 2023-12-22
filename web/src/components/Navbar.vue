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
        <div v-show="filterConfig.search.enable" class="mr-[60px] flex-1 sm:mr-0">
          <input
            class="box-border h-[60px] w-full border-x border-gray-400/50 bg-transparent px-4 outline-none"
            type="text"
            placeholder="图片id/图片标题/作者id/作者昵称/标签"
            :value="filterConfig.search.value"
            @input="handleSearchInput"
          >
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
        class="mx-[60px] h-[60px] select-none text-center text-lg leading-[60px]"
        @click="navToTop"
      >
        <span class="text-[#0398fa]">Pixiv</span>Collection
        <svg class="inline-block cursor-pointer" style="vertical-align: -0.5em;" viewBox="0 0 1024 1024" width="30" height="30"><path d="M698.8 337.6H325.2c-18.4 0-33.5-14.4-33.5-32s15.1-32 33.5-32h373.7c18.4 0 33.5 14.4 33.5 32-0.1 17.6-15.1 32-33.6 32z" fill="" p-id="4308"></path><path d="M508.4 547.8l1.8-1.8-1.8 1.8zM508.2 545.8l2.2 2.2c-0.7-0.8-1.4-1.5-2.2-2.2zM511.1 508.7l1.8 1.8-1.8-1.8z" fill="#FFFFFF" p-id="4309"></path><path d="M510.9 510.7l2.2-2.2c-0.8 0.7-1.5 1.4-2.2 2.2z" fill="#FFFFFF" p-id="4310"></path><path d="M544 472.4v246c0 17.6-14.4 32-32 32s-32-14.4-32-32v-246c0-17.6 14.4-32 32-32s32 14.4 32 32z" fill="" p-id="4311"></path><path d="M511.9 379c-8.3 0-15.8 3.1-21.5 8.3l-2.2 2.2-21.5 21.5L311 566.7c-12.4 12.4-12.4 32.8 0 45.3 12.4 12.4 32.8 12.4 45.3 0L512 456.2l155.8 155.7c12.4 12.4 32.8 12.4 45.3 0 12.4-12.4 12.4-32.8-0.1-45.2L557.3 411l-21.8-21.8-1.8-1.8c-5.7-5.3-13.4-8.5-21.8-8.4z" fill=""></path></svg>
      </div>
      <div class="absolute right-0 top-0 hidden lg:flex">
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
        <button
          class="h-[60px] w-[60px] hover:bg-gray-400/20"
          @click="store.toggleFullscreen"
        >
          <IconShrink v-if="isFullscreen" class="mx-auto h-5 w-5" />
          <IconExpand v-else class="mx-auto h-5 w-5" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useDebounceFn } from '@vueuse/core'
import { useStore } from '@/store'
import { LINK_GITHUB, NAVBAR_HIDE_DISTANCE } from '@/config'

const store = useStore()
const { preferColorScheme, showSidebar, showNav, imageViewer, isFullscreen, filterConfig } = toRefs(store)

const updateSearchStr = useDebounceFn(value => {
  store.updateSeatchValue(value)
}, 800)

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

function handleSearchInput(e: Event) {
  const value = (e.target as HTMLInputElement).value
  updateSearchStr(value)
}
</script>
