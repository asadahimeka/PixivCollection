<template>
  <div
    class="group absolute overflow-hidden bg-gray-100 transition-all duration-300 dark:bg-[#242424]"
    :class="{
      'rounded-[12px] border dark:border-[#505050] ': config.border,
      'shadow-[0_3px_10px_1px_rgba(0,0,0,0.20)]': config.shadow && config.border,
    }"
  >
    <div
      class="relative"
      :style="{
        height: `${imageHeight}px`,
      }"
    >
      <Transition name="fade-slow">
        <div
          v-show="!imageLoaded"
          class="absolute h-full w-full"
          :style="{
            background: randomBg(),
          }"
        ></div>
      </Transition>
      <img
        class="w-full cursor-pointer"
        :src="imageLoad ? getImageSrc(imageData) : ''"
        @load="handleImageLoaded"
        @click="$emit('viewImage', imageIndex)"
      >
    </div>
    <Transition name="fade">
      <div
        v-if="config.infoAtBottom"
        class="w-full px-2 py-1"
      >
        <p
          class="cursor-pointer truncate font-bold transition-colors hover:text-blue-500"
          @click="openPixivIllust(imageData.id)"
        >
          {{ imageData.title }}
        </p>
        <p class="flex cursor-pointer items-center">
          <span
            class="truncate text-sm transition-colors hover:text-blue-500"
            @click="openPixivUser(imageData.author.id)"
          >{{ imageData.author.name }}</span>
          <IconFunnelSolid
            class="ml-1 inline-block h-3 w-3 transition-colors hover:text-blue-500"
            @click="$emit('filterAuthor', imageIndex)"
          />
        </p>
        <p class="mx-[-4px] h-[50px] overflow-y-auto">
          <span v-if="imageData.x_restrict === 1" class="float-left m-0.5 rounded-sm !bg-red-500/60 px-1 text-xs dark:bg-gray-200/10">R18</span>
          <span v-if="imageData.x_restrict === 2" class="float-left m-0.5 rounded-sm !bg-red-500/60 px-1 text-xs dark:bg-gray-200/10">R18G</span>
          <span v-if="imageData.isAI" class="float-left m-0.5 rounded-sm !bg-blue-500/60  px-1 text-xs dark:bg-gray-200/10">AI 生成</span>
          <span
            v-for="tag, idx in imageData.tags"
            v-show="!tag.name.includes('users入り') || config.tagIncludeBookmark" :key="idx"
            class="float-left m-0.5 cursor-pointer rounded-sm bg-black/10 px-1 text-xs dark:bg-gray-200/10"
            @click.stop="handleClickTag(tag.name)"
          >
            {{ config.tagTranslation ? tag.translated_name || tag.name : tag.name }}
          </span>
        </p>
        <p class="mt-0.5 flex items-center whitespace-nowrap text-left text-xs text-gray-500">
          {{ imageData.id }}
          {{ `p${imageData.part}` }}
          {{ `${imageData.size[0]}×${imageData.size[1]}` }}
          {{ `L${imageData.sanity_level}` }}
          ♥{{ imageData.bookmark }}
        </p>
      </div>
    </Transition>
    <div
      v-if="imageCount > 1"
      class="absolute right-1 top-1 flex items-center rounded-full bg-black/50 px-2 py-0.5 text-sm text-white"
    >
      <IconStack class="mr-1 h-3 w-3" />
      {{ imageCount }}
    </div>
    <div
      v-if="!config.infoAtBottom"
      class="absolute top-0 h-full w-full cursor-pointer bg-black/50 px-2 pt-2 text-sm text-white opacity-0 transition-all duration-300 group-hover:opacity-100"
      @click="$emit('viewImage', imageIndex)"
    >
      <p class="flex items-center whitespace-nowrap">
        <IconTitle class="mr-1 inline-block h-4 w-4" />
        <span
          class="overflow-hidden text-ellipsis transition-colors hover:text-blue-500"
          @click.stop="openPixivIllust(imageData.id)"
        >{{ imageData.title }}</span>
      </p>
      <p class="flex items-center whitespace-nowrap">
        <IconUser class="mr-1 inline-block h-4 w-4" />
        <span
          class="overflow-hidden text-ellipsis transition-colors hover:text-blue-500"
          @click.stop="openPixivUser(imageData.author.id)"
        >{{ imageData.author.name }}</span>
        <IconFunnelSolid
          class="ml-1 inline-block h-3 w-3 transition-colors hover:text-blue-500"
          @click.stop="$emit('filterAuthor', imageIndex)"
        />
      </p>
      <p>
        <IconTag class="mr-1 inline-block h-4 w-4" />
        <span v-if="imageData.x_restrict === 1" class="my-0.5 mr-1 inline-block rounded-sm !bg-red-500/60 px-1 text-xs">R18</span>
        <span v-if="imageData.x_restrict === 2" class="my-0.5 mr-1 inline-block rounded-sm !bg-red-500/60 px-1 text-xs">R18G</span>
        <span v-if="imageData.isAI" class="my-0.5 mr-1 inline-block rounded-sm !bg-blue-500/60 px-1 text-xs">AI 生成</span>
        <span
          v-for="tag, idx in imageData.tags"
          v-show="!tag.name.includes('users入り') || config.tagIncludeBookmark"
          :key="idx"
          class="my-0.5 mr-1 inline-block rounded-sm bg-black/30 px-1 text-xs"
          :class="tag.name === 'R-18' ? 'bg-red-500/80' : ''"
        >
          {{ config.tagTranslation ? tag.translated_name || tag.name : tag.name }}
        </span>
      </p>
      <p class="mt-1 text-xs">
        {{ `${imageData.id} p${imageData.part} ♥${imageData.bookmark} ${imageData.size[0]}×${imageData.size[1]} L${imageData.sanity_level}` }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/tauri'

import { IMAGE_FORMAT_THUMBNAIL, IMAGE_PATH_THUMBNAIL, MASONRY_LOAD_DELAY } from '@/config'
import { useStore } from '@/store'
import { openPixivIllust, openPixivUser } from '@/utils'

const props = defineProps<{
  imageData: Image
  imageIndex: number
  imageHeight: number
  imageCount: number
  config: {
    infoAtBottom: boolean
    tagIncludeBookmark: boolean
    tagTranslation: boolean
    shadow: boolean
    border: boolean
  }
}>()

defineEmits(['viewImage', 'filterAuthor'])

let timer: NodeJS.Timeout | null = null

const store = useStore()
const imageLoad = ref(false)
const imageLoaded = ref(false)

const imageIdxStr = `${props.imageData.id * 100 + props.imageData.part}`

onMounted(() => {
  if (store.imagesLoaded.has(imageIdxStr)) {
    imageLoad.value = true
    return
  }
  timer = setTimeout(() => {
    imageLoad.value = true
    timer = null
  }, MASONRY_LOAD_DELAY)
})

onUnmounted(() => {
  if (timer) { clearTimeout(timer) }
})

function handleImageLoaded() {
  imageLoaded.value = true
  store.imagesLoaded.add(imageIdxStr)
}

const { filterConfig } = toRefs(store)
function handleClickTag(tagName: string) {
  if (filterConfig.value.tag.enable && filterConfig.value.tag.name === tagName) {
    filterConfig.value.tag.enable = false
    filterConfig.value.tag.name = ''
  } else {
    filterConfig.value.tag.name = tagName
    filterConfig.value.tag.enable = true
  }
}

function getImageSrc(img: Image) {
  // return img.images.m.replace('i.pximg.net', 'pximg.cocomi.eu.org')

  // eslint-disable-next-line no-control-regex
  const title = img.title.replace(/[\x00-\x1F\x7F]/g, '').replace(/[/\\:*?"<>|.&$]/g, '')
  const fileName = `(${img.id})${title}${img.part == 0 ? '' : `_p${img.part}`}${img.ext}`
  return convertFileSrc(`E:\\Pictures\\Pixiv\\[bookmark] Public\\${fileName}`)
}

function randomBg() {
  const getRandomRangeNum = (min: number, max: number) => min + Math.floor(Math.random() * (max - min))
  const leftHue = getRandomRangeNum(0, 360)
  const bottomHue = getRandomRangeNum(0, 360)
  const css = [
    'linear-gradient(to left bottom,hsl(',
    leftHue,
    ', 50%, 85%) 0%,hsl(',
    bottomHue,
    ', 50%, 85%) 100%)',
  ]
  return css.join('')
}
</script>
