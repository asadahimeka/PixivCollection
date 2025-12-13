// import { LINK_PIXIV_ARTWORK, LINK_PIXIV_USER } from '@/config'

export function formatBytes(bytes: number) {
  if (bytes === 0) { return '0 B' }
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / k ** i).toFixed(2)} ${sizes[i]}`
}

export function openPixivIllust(pid: number) {
  // window.open(LINK_PIXIV_ARTWORK.replace('{id}', pid.toString()), '_blank')
  (parent as any).__pxcl.routerPush(`/i/${pid}`)
}

export function openPixivUser(uid: number) {
  // window.open(LINK_PIXIV_USER.replace('{id}', uid.toString()), '_blank')
  (parent as any).__pxcl.routerPush(`/u/${uid}`)
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

const PXIMG_BASE = (() => {
  try {
    const json = localStorage.getItem('PXIMG_PROXY')
    if (!json) return 'pximg.cocomi.eu.org'
    return JSON.parse(json).data as string
  } catch (err) {
    return 'pximg.cocomi.eu.org'
  }
})()

const handleRecoverSrc = (src: string, img: Image) => src?.includes('common/images/limit')
  ? `https://pximg.cocomi.eu.org/_pid_/${img.id}_${img.part}_m`
  : src

export function getImageMediumSrc(img: Image) {
  const src = img.images?.l
    .replace('i.pximg.net', PXIMG_BASE)
    .replace(/\/c\/\d+x\d+(_\d+)?\//g, '/c/1200x1200_90_webp/')
  return handleRecoverSrc(src, img)
}

const isOriginalSrc = !!localStorage.getItem('__PXCT_DTL_ORI_SRC')
export function getImageLargeSrc(img: Image) {
  if (img.images?.o.includes('_ugoira')) {
    return `https://ugoira-mp4-dl.cocomi.eu.org/${img.id}.mp4`
  }
  const src = isOriginalSrc
    ? img.images?.o.replace('i.pximg.net', PXIMG_BASE)
    : img.images?.l
      .replace('i.pximg.net', PXIMG_BASE)
      .replace(/\/c\/\d+x\d+(_\d+)?\//g, '/c/1200x1200_90_webp/')

  return handleRecoverSrc(src, img)
}

export function getImageOriginalSrc(img: Image) {
  const src = img.images?.o.replace('i.pximg.net', PXIMG_BASE)
  return handleRecoverSrc(src, img)
}

export async function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

const aiTags = [
  'ai',
  'ai生成',
  'ai生成作品',
  'ai作画',
  'aiイラスト',
  'aigenerated',
  'ai-generated',
  'ai-assisted',
  'ai辅助',
  'aiアシスタンス',
  'ai_generated',
  'aiartwork',
  'aigirl',
  'ai作品',
  'ai生成イラスト',
  'ai画像',
  'ai绘画',
  'novelai',
  'novelaidiffusion',
  'stablediffusion',
]
function isAiIllust(artwork: any) {
  return artwork.illust_ai_type == 2 || !!artwork.tags?.some((e: any) => aiTags.includes(e.name?.toLowerCase()))
}

export function transformResData(data: any[]) {
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
        isAI: isAiIllust(json),
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
        isAI: isAiIllust(json),
      })))
    }
  }
  return results
}
