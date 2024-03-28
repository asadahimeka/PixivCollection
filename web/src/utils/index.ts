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
  if (!store.masonryConfig.useLocalImage || !imgDir) {
    return img.images?.m.replace('i.pximg.net', 'i.pixiv.pics') || `https://f.pixiv.pics/pid/${img.id}?size=medium`
  }

  // eslint-disable-next-line no-control-regex
  const title = img.title.replace(/[\x00-\x1F\x7F]/g, '').replace(/[/\\:*?"<>|.&$]/g, '')
  const fileName = `(${img.id})${title}${img.len == 1 ? '' : `_p${img.part}`}.${img.ext}`
  return convertFileSrc(`${imgDir}${/[\\/]$/.test(imgDir) ? '' : '/'}${fileName}`)
}

export function getImageLargeSrc(store: any, img: Image) {
  if (!store.masonryConfig.useLocalImage || !imgDir) {
    return img.images?.l.replace('i.pximg.net', 'i.pixiv.pics').replace(/\/c\/\d+x\d+_\d+(_webp)?\//, '/') || `https://f.pixiv.pics/pid/${img.id}?size=large&p=${img.part}`
    // return img.images.o.replace('i.pximg.net', 'i.pixiv.pics')
  }

  // eslint-disable-next-line no-control-regex
  const title = img.title.replace(/[\x00-\x1F\x7F]/g, '').replace(/[/\\:*?"<>|.&$]/g, '')
  const fileName = `(${img.id})${title}${img.len == 1 ? '' : `_p${img.part}`}.${img.ext}`
  return convertFileSrc(`${imgDir}${/[\\/]$/.test(imgDir) ? '' : '/'}${fileName}`)
}

export function getImageOriginalSrc(img: Image) {
  return img.images?.o.replace('i.pximg.net', 'i.pixiv.pics') || `https://f.pixiv.pics/pid/${img.id}?p=${img.part}`
}
