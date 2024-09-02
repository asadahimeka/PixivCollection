import { Settings } from '@orilight/vue-settings'
import { useFullscreen, usePreferredColorScheme } from '@vueuse/core'
import { defineStore } from 'pinia'

import { getImageLargeSrc } from '@/utils'

const w = window as any
export const useStore = defineStore('main', {
  state: () => ({
    showSidebar: false,
    showNav: true,

    imagesFiltered: [] as Image[],
    imagesLoaded: new Set(),
    curPageCursor: 0,
    loadEnd: false,

    fullCounts: {
      total: 0,
      illustCount: 0,
      authorCount: 0,
      tagCount: 0,
    },
    filteredCounts: {
      total: 0,
      illustCount: 0,
      authorCount: 0,
      tagCount: 0,
    },

    fullscreen: useFullscreen(document.documentElement),
    preferColorScheme: 'auto' as 'auto' | 'light' | 'dark',
    browserColorScheme: usePreferredColorScheme(),

    settings: new Settings('PXCT'),

    imageViewer: {
      show: false,
      showInfo: true,
      info: null as Image | null,
      prev: () => {},
      next: () => {},
      index: -1,
    },
    masonryConfig: {
      col: -1,
      gap: 20,
      imageMinWidth: 240,
      containerFullWidth: true,
      mergeSameIdImage: true,
      infoAtBottom: false,
      showTagTranslation: true,
      virtualListEnable: true,
      showShadow: false,
      imageSortBy: 'id_desc' as 'id_desc' | 'id_asc' | 'bookmark_desc',
      useLocalImage: true,
      useFancybox: false,
      sliceLocalImages: true,
      loadImagesJsonByLocalHttp: true,
      loadImageByLocalHttp: false,
    },
    filterConfig: {
      search: {
        enable: false,
        value: '',
      },
      year: {
        enable: false,
        value: 0,
      },
      tag: {
        enable: false,
        name: '',
        includeBookmark: false,
        includeRatherThan: 5,
      },
      author: {
        enable: false,
        id: -1,
      },
      shape: {
        enable: false,
        value: '',
      },
      size: {
        enable: false,
        width: { max: null, min: null },
        height: { max: null, min: null },
      },
      bookmark: {
        enable: false,
        min: 0,
      },
      restrict: {
        maxSanityLevel: 6,
        r18: 'hidden' as 'hidden' | 'show' | 'only',
      },
    },
  }),
  getters: {
    isFullscreen(): boolean {
      return this.fullscreen.isFullscreen
    },
    colorScheme(): 'light' | 'dark' {
      if (this.preferColorScheme === 'auto') {
        if (this.browserColorScheme === 'no-preference') { return 'light' }
        return this.browserColorScheme
      }
      return this.preferColorScheme
    },
    imageFilter() {
      return (image: Image) => {
        // 过滤_不健全度
        if (this.filterConfig.restrict.r18 === 'hidden') {
          if (image.x_restrict >= 1) { return false }
        } else if (this.filterConfig.restrict.r18 === 'only') {
          if (image.x_restrict < 1) { return false }
        }
        if (this.filterConfig.restrict.maxSanityLevel) {
          if (image.sanity_level > this.filterConfig.restrict.maxSanityLevel) { return false }
        }
        // 搜索
        if (this.filterConfig.search.enable) {
          if (this.filterConfig.search.value.trim() !== '' && image.searchStr !== undefined) {
            if (!image.searchStr.includes(this.filterConfig.search.value.trim().toLowerCase())) { return false }
          }
          return true
        }
        if (this.filterConfig.bookmark.enable) {
          if (this.filterConfig.bookmark.min === -1) {
            if (image.bookmark !== -1) { return false }
          }
          if (image.bookmark < this.filterConfig.bookmark.min) { return false }
        }
        // 过滤_年份
        if (this.filterConfig.year.enable) {
          const year = Number(image.created_at.split('-')[0])
          if (this.filterConfig.year.value === 1) {
            if (year > 2000) { return false }
          } else if (year !== this.filterConfig.year.value) {
            return false
          }
        }
        // 过滤_作者
        if (this.filterConfig.author.enable) {
          if (image.author.id !== this.filterConfig.author.id) { return false }
        }
        // 过滤_标签
        if (this.filterConfig.tag.enable) {
          if ((image.tags.find(tag => {
            if (tag.name === this.filterConfig.tag.name) { return tag }
            return undefined
          })) === undefined) { return false }
        }
        // 过滤_形状
        if (this.filterConfig.shape.enable) {
          if (this.filterConfig.shape.value.startsWith('ratio-')) {
            const _ratioStr = this.filterConfig.shape.value.substring(6).split(':').map(str => Number(str))
            const ratio = _ratioStr[0] / _ratioStr[1]
            if (image.size[0] / image.size[1] >= ratio / 0.9 || image.size[0] / image.size[1] <= ratio * 0.9) { return false }
          } else if (image.size[0] / image.size[1] < 0.9 || image.size[0] / image.size[1] > 1.1) {
            if (this.filterConfig.shape.value === 'square') { return false }
            if (image.size[0] > image.size[1]) {
              if (this.filterConfig.shape.value === 'vertical') { return false }
            } else {
              if (this.filterConfig.shape.value === 'horizontal') { return false }
            }
          } else {
            if (this.filterConfig.shape.value === 'vertical' || this.filterConfig.shape.value === 'horizontal') { return false }
          }
        }
        // 过滤_尺寸
        if (this.filterConfig.size.enable) {
          const { max: wMax, min: wMin } = this.filterConfig.size.width
          if (wMax) {
            if (image.size[0] > wMax) { return false }
          }
          if (wMin) {
            if (image.size[0] < wMin) { return false }
          }
          const { max: hMax, min: hMin } = this.filterConfig.size.height
          if (hMax) {
            if (image.size[1] > hMax) { return false }
          }
          if (hMin) {
            if (image.size[1] < hMin) { return false }
          }
        }
        return true
      }
    },
  },
  actions: {
    updateFullCounts() {
      this.fullCounts.total = w.__fullImages__.length
      this.fullCounts.illustCount = new Set(w.__fullImages__.map((i: Image) => i.id)).size
      this.fullCounts.authorCount = new Set(w.__fullImages__.map((i: Image) => i.author.id)).size
      this.fullCounts.tagCount = new Set(w.__fullImages__.flatMap((i: Image) => i.tags.map(t => t.name))).size
    },
    updateFilteredCounts() {
      this.filteredCounts.total = w.__filteredImages__.length
      this.filteredCounts.illustCount = new Set(w.__filteredImages__.map((i: Image) => i.id)).size
      this.filteredCounts.authorCount = new Set(w.__filteredImages__.map((i: Image) => i.author.id)).size
      this.filteredCounts.tagCount = new Set(w.__filteredImages__.flatMap((i: Image) => i.tags.map(t => t.name))).size
    },
    loadFilteredImages() {
      const res: Image[] = []
      const fullList = w.__fullImages__
      const len = fullList.length
      for (let i = 0; i < len; i++) {
        const item = fullList[i]
        if (this.imageFilter(item)) { res.push(item) }
      }
      this.imagesFiltered = res
    },
    loadImagesByPage(isFirstLoad = false) {
      if (isFirstLoad) {
        w.__filteredImages__ = w.__fullImages__
        this.updateFilteredCounts()
      } else if (this.curPageCursor == 0) {
        const res: Image[] = []
        const fullList = w.__fullImages__
        const len = fullList.length
        for (let i = 0; i < len; i++) {
          const item = fullList[i]
          if (this.imageFilter(item)) { res.push(item) }
        }
        w.__filteredImages__ = res
        this.updateFilteredCounts()
      }
      if (w.__filteredImages__.length == this.imagesFiltered.length) {
        this.loadEnd = true
        return
      }
      this.loadEnd = false
      let res: any[] = w.__filteredImages__.slice(this.curPageCursor, this.curPageCursor + 30)
      const lastEl = res[res.length - 1]
      if (lastEl.len > 1) {
        const newLen = this.imagesFiltered.length + res.length
        res = res.concat(
          w.__filteredImages__.slice(
            newLen,
            newLen + lastEl.len - lastEl.part - 1,
          ),
        )
      }
      this.imagesFiltered = this.imagesFiltered.concat(res)
      this.curPageCursor += res.length
    },
    openImageViewer(image: Image, prev: () => void, next: () => void, index: number): void {
      this.imageViewer.show = true
      this.imageViewer.info = image
      this.imageViewer.prev = prev
      this.imageViewer.next = next
      this.imageViewer.index = index
    },
    closeImageViewer(): void {
      this.imageViewer.show = false
    },
    updateSeatchValue(value: string): void {
      this.filterConfig.search.value = value
    },
    toggleColorScheme(): void {
      switch (this.preferColorScheme) {
        case 'auto':
          this.preferColorScheme = 'light'
          break
        case 'light':
          this.preferColorScheme = 'dark'
          break
        case 'dark':
          this.preferColorScheme = 'auto'
          break
        default:
          this.preferColorScheme = 'auto'
          break
      }
    },
    toggleFullscreen(): void {
      this.fullscreen.toggle()
    },
    toggleSearch(): void {
      if (this.filterConfig.search.enable) {
        this.updateSeatchValue('')
      } else {
        for (let i = 0, len = w.__fullImages__.length; i < len; i++) {
          const image = w.__fullImages__[i] as Image
          if (image.searchStr === undefined) {
            image.searchStr = (
              image.id
              + image.title
              + image.author.id
              + image.author.name
              + image.tags
                .map(
                  tag => tag.translated_name
                    ? tag.name + tag.translated_name
                    : tag.name,
                ).join()
            ).toLowerCase()
          }
        }
      }
      this.filterConfig.search.enable = !this.filterConfig.search.enable
    },
    filterAuthor(idx: number): void {
      const authorId = this.imagesFiltered[idx].author.id
      if (this.filterConfig.author.enable && this.filterConfig.author.id === authorId) {
        this.filterConfig.author.enable = false
        this.filterConfig.author.id = -1
        return
      }
      this.filterConfig.author.id = authorId
      this.filterConfig.author.enable = true

      if (this.filterConfig.search.enable) { this.toggleSearch() }
    },
    viewImage(idx: number): void {
      if (idx < 0 || idx >= this.imagesFiltered.length) { return }
      const actItem = this.imagesFiltered[idx]
      if (this.masonryConfig.useFancybox) {
        const list: Image[] = [actItem]
        if (actItem.len > 1) {
          for (let i = 1; i < actItem.len; i++) {
            list.push(this.imagesFiltered[idx + i])
          }
        }
        w.Fancybox.show(list.map(e => ({ src: getImageLargeSrc(this, e) })), {
          startIndex: 0,
          Thumbs: { showOnStart: false },
          Carousel: { infinite: false },
          Toolbar: {
            display: {
              right: ['iterateZoom', 'rotateCCW', 'rotateCW', 'flipX', 'flipY', 'fullscreen', 'close'],
            },
          },
        })
      } else {
        this.openImageViewer(
          actItem,
          () => {
            this.viewImage(idx - 1)
          },
          () => {
            this.viewImage(idx + 1)
          },
          idx,
        )
      }
    },
    sortImages(): void {
      w.__fullImages__.sort((a: Image, b: Image) => {
        if (a.id === b.id) { return a.part - b.part }
        if (this.masonryConfig.imageSortBy === 'bookmark_desc') { return b.bookmark - a.bookmark }
        if (this.masonryConfig.imageSortBy === 'id_desc') { return b.id - a.id }
        return (a.id - b.id)
      })
      if (this.masonryConfig.sliceLocalImages) {
        this.curPageCursor = 0
        this.loadEnd = false
        this.imagesFiltered = []
        this.loadImagesByPage()
      } else {
        this.loadFilteredImages()
      }
    },
  },
})
