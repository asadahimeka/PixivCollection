import { convertFileSrc } from '@tauri-apps/api/tauri'

import { LINK_PIXIV_ARTWORK, LINK_PIXIV_USER } from '@/config'

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

  if (store.masonryConfig.useLocalImage && store.masonryConfig.loadImageByLocalHttp) {
    if (img.images?.o.includes('_ugoira')) {
      return `http://localhost:32154/bookmark_ugoira/${fileNameWoExt}.mp4`
    }
    return `http://localhost:32154/bookmark_webp/${fileNameWoExt}.webp`
  }

  if (!store.masonryConfig.useLocalImage || !imgDir) {
    return img.images?.m.replace('i.pximg.net', 'pximg.cocomi.eu.org') || `https://pximg.cocomi.eu.org/_pid_/${img.id}_${img.part}_m`
  }

  const dir = `${imgDir}${/[\\/]$/.test(imgDir) ? '' : '/'}`

  if (img.images?.o.includes('_ugoira')) {
    return convertFileSrc(`${dir}bookmark_ugoira/${fileNameWoExt}.mp4`)
  }
  return convertFileSrc(`${dir}bookmark_webp/${fileNameWoExt}.webp`)
}

export function getImageLargeSrc(store: any, img: Image) {
  // eslint-disable-next-line no-control-regex
  const title = img.title.replace(/[\x00-\x1F\x7F]/g, '').replace(/[/\\:*?"<>|.&$]/g, '')
  const fileNameWoExt = `(${img.id})${title}${img.len == 1 ? '' : `_p${img.part}`}`
  // const fileName = `(${img.id})${title}${img.len == 1 ? '' : `_p${img.part}`}.${img.ext}`

  if (store.masonryConfig.useLocalImage && store.masonryConfig.loadImageByLocalHttp) {
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
    return img.images?.l.replace('i.pximg.net', 'pximg.cocomi.eu.org').replace(/\/c\/\d+x\d+_\d+(_webp)?\//, '/') || `https://pximg.cocomi.eu.org/_pid_/${img.id}_${img.part}_l`
    // return img.images.o.replace('i.pximg.net', 'pximg.cocomi.eu.org')
  }

  const dir = `${imgDir}${/[\\/]$/.test(imgDir) ? '' : '/'}`

  if (img.images?.o.includes('_ugoira')) {
    return convertFileSrc(`${dir}bookmark_ugoira/${fileNameWoExt}.mp4`)
  }
  return convertFileSrc(`${dir}bookmark_webp/${fileNameWoExt}.webp`)
}

export function getImageOriginalSrc(img: Image) {
  return img.images?.o.replace('i.pximg.net', 'pximg.cocomi.eu.org') || `https://pximg.cocomi.eu.org/_pid_/${img.id}_${img.part}_o`
}
