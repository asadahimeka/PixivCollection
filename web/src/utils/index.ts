import { convertFileSrc } from '@tauri-apps/api/tauri'

import { LINK_PIXIV_ARTWORK, LINK_PIXIV_USER } from '@/config'

export function formatBytes(bytes: number) {
  if (bytes === 0) { return '0 B' }
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / k ** i).toFixed(2)} ${sizes[i]}`
}

export function openPixivIllust(pid: number) {
  window.open(LINK_PIXIV_ARTWORK.replace('{id}', pid.toString()), '_blank')
}

export function openPixivUser(uid: number) {
  window.open(LINK_PIXIV_USER.replace('{id}', uid.toString()), '_blank')
}

export function exportFile(data: string, filename = 'export-{ts}.json') {
  const blob = new Blob([data], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename.replace('{ts}', Date.now().toString())
  a.click()
  URL.revokeObjectURL(url)
}

const { imgDir } = (window as any).__CONFIG__
export function getImageMediumSrc(store: any, img: Image) {
  // eslint-disable-next-line no-control-regex
  const title = img.title.replace(/[\x00-\x1F\x7F]/g, '').replace(/[/\\:*?"<>|.&$]/g, '')
  const fileNameWoExt = `(${img.id})${title}${img.len == 1 ? '' : `_p${img.part}`}`
  // const fileName = `${fileNameWoExt}.${img.ext}`

  if (store.masonryConfig.useLocalImage  && store.masonryConfig.loadImageByLocalHttp) {
    if (img.images?.o.includes('_ugoira')) {
      return `http://localhost:32154/bookmark_ugoira/${fileNameWoExt}.mp4`
    }
    return `http://localhost:32154/bookmark_webp/${fileNameWoExt}.webp`
  }

  if (!store.masonryConfig.useLocalImage || !imgDir) {
    return img.images?.m.replace('i.pximg.net', 'pximg.cocomi.eu.org') || `https://f.cocomi.eu.org/pid/${img.id}?size=medium`
  }

  if (img.images?.o.includes('_ugoira')) {
    return convertFileSrc(`E:/Pictures/Pixiv/bookmark_ugoira/${fileNameWoExt}.mp4`)
  }
  return convertFileSrc(`${imgDir}${/[\\/]$/.test(imgDir) ? '' : '/'}${fileNameWoExt}.webp`)
}

export function getImageLargeSrc(store: any, img: Image) {
  // eslint-disable-next-line no-control-regex
  const title = img.title.replace(/[\x00-\x1F\x7F]/g, '').replace(/[/\\:*?"<>|.&$]/g, '')
  const fileNameWoExt = `(${img.id})${title}${img.len == 1 ? '' : `_p${img.part}`}`
  // const fileName = `(${img.id})${title}${img.len == 1 ? '' : `_p${img.part}`}.${img.ext}`

  if (store.masonryConfig.useLocalImage  && store.masonryConfig.loadImageByLocalHttp) {
    if (img.images?.o.includes('_ugoira')) {
      return `http://localhost:32154/bookmark_ugoira/${fileNameWoExt}.mp4`
    }
    return `http://localhost:32154/bookmark_webp/${fileNameWoExt}.webp`
  }

  if (!store.masonryConfig.useLocalImage || !imgDir) {
    if (img.images?.o.includes('_ugoira')) {
      return `https://ugoira-mp4-dl.cocomi.eu.org/${img.id}.mp4`
      // return `https://hibiapi.cocomi.eu.org/api/ugoira/${img.id}.mp4`
    }
    return img.images?.l.replace('i.pximg.net', 'pximg.cocomi.eu.org').replace(/\/c\/\d+x\d+_\d+(_webp)?\//, '/') || `https://f.cocomi.eu.org/pid/${img.id}?size=large&p=${img.part}`
    // return img.images.o.replace('i.pximg.net', 'pximg.cocomi.eu.org')
  }

  if (img.images?.o.includes('_ugoira')) {
    return convertFileSrc(`E:/Pictures/Pixiv/bookmark_ugoira/${fileNameWoExt}.mp4`)
  }
  return convertFileSrc(`${imgDir}${/[\\/]$/.test(imgDir) ? '' : '/'}${fileNameWoExt}.webp`)
}

export function getImageOriginalSrc(img: Image) {
  return img.images?.o.replace('i.pximg.net', 'pximg.cocomi.eu.org') || `https://f.cocomi.eu.org/pid/${img.id}?p=${img.part}`
}
