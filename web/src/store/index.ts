import { Settings } from '@orilight/vue-settings'
import { useFullscreen, usePreferredColorScheme } from '@vueuse/core'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/tauri'

import { getImageLargeSrc } from '@/utils'

export interface QueryResult {
  images: Image[]
}

const w = window as any
export const useStore = defineStore('main', {
  state: () => ({
    showSidebar: false,
    showNav: true,

    imagesFiltered: shallowRef<Image[]>([]),
    imagesLoaded: new Set(),
    curPageCursor: 0,
    loadEnd: false,
    isFiltering: false,

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
      gap: 10,
      imageMinWidth: 280,
      containerFullWidth: true,
      mergeSameIdImage: true,
      infoAtBottom: true,
      showTagTranslation: false,
      virtualListEnable: true,
      showShadow: false,
      imageSortBy: 'original' as 'original' | 'id_desc' | 'id_asc' | 'bookmark_desc' | 'bookmark_asc' | 'created_at_desc' | 'created_at_asc' | 'view_desc' | 'view_asc' | 'random',
      useLocalImage: true,
      useFancybox: false,
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
        r18: 'show' as 'hidden' | 'show' | 'only',
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
  },
  actions: {
    async loadImagesByPage(isFirstLoad = false) {
      if (isFirstLoad) {
        this.curPageCursor = 0
        this.loadEnd = false
        this.imagesFiltered = []
        this.isFiltering = true
      }
      try {
        const query = this.buildFilterQuery()
        query.offset = this.curPageCursor
        query.limit = 60
        query.sort_by = this.masonryConfig.imageSortBy
        const result = await invoke<any>('query_images', {
          query,
        })
        // Transform flat Rust fields → frontend nested format
        const images: Image[] = result.images.map((img: any) => ({
          id: img.id,
          part: img.part,
          len: img.len,
          title: img.title,
          ext: img.ext,
          size: [img.width, img.height] as [number, number],
          author: { id: img.author_id, name: img.author_name, account: img.author_account },
          tags: img.tags ?? [],
          created_at: img.created_at,
          sanity_level: img.sanity_level,
          x_restrict: img.x_restrict,
          dominant_color: '',
          bookmark: img.bookmark,
          view: img.view,
          images: { s: img.img_s, m: img.img_m, l: img.img_l, o: img.img_o },
          isAI: img.is_ai,
        }))
        if (isFirstLoad) {
          this.imagesFiltered = images
        } else {
          this.imagesFiltered = this.imagesFiltered.concat(images)
        }
        this.curPageCursor += result.images.length
        this.loadEnd = result.images.length < (query.limit ?? 60)

        if (isFirstLoad || this.curPageCursor === 0) {
          this.fetchFilteredCounts()
        }
      } finally {
        if (isFirstLoad) this.isFiltering = false
      }
    },
    async fetchFilteredCounts() {
      const query = this.buildFilterQuery()
      try {
        const result = await invoke<any>('query_image_counts', { query })
        this.filteredCounts.total = result.total
        this.filteredCounts.illustCount = result.illust_count
        this.filteredCounts.authorCount = result.author_count
        this.filteredCounts.tagCount = result.tag_count
      } catch (e) {
        console.warn('query_image_counts failed (preserving previous values):', e)
      }
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
    updateSearchValue(value: string): void {
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
        this.updateSearchValue('')
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
    async sortImages() {
      this.curPageCursor = 0
      this.loadEnd = false
      this.imagesFiltered = []
      await this.loadImagesByPage(true)
    },
    buildFilterQuery(): Record<string, any> {
      const q: Record<string, any> = {}
      if (this.filterConfig.search.enable && this.filterConfig.search.value) {
        q.search = this.filterConfig.search.value
      }
      if (this.filterConfig.year.enable && this.filterConfig.year.value) {
        q.year = this.filterConfig.year.value
      }
      if (this.filterConfig.tag.enable && this.filterConfig.tag.name) {
        q.tag = this.filterConfig.tag.name
      }
      if (this.filterConfig.author.enable && this.filterConfig.author.id !== -1) {
        q.author_id = this.filterConfig.author.id
      }
      if (this.filterConfig.shape.enable && this.filterConfig.shape.value) {
        q.shape = this.filterConfig.shape.value
      }
      if (this.filterConfig.size.enable) {
        if (this.filterConfig.size.width.min !== null) { q.width_min = this.filterConfig.size.width.min }
        if (this.filterConfig.size.width.max !== null) { q.width_max = this.filterConfig.size.width.max }
        if (this.filterConfig.size.height.min !== null) { q.height_min = this.filterConfig.size.height.min }
        if (this.filterConfig.size.height.max !== null) { q.height_max = this.filterConfig.size.height.max }
      }
      if (this.filterConfig.bookmark.enable) {
        q.bookmark_min = this.filterConfig.bookmark.min
      }
      q.r18 = this.filterConfig.restrict.r18
      q.max_sanity_level = this.filterConfig.restrict.maxSanityLevel
      return q
    },
  },
})
