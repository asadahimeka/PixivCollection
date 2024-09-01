import { join } from 'node:path'
import fs from 'fs-extra'
import uniqBy from 'lodash.uniqby'

const dataDir = join(import.meta.dirname, '../data')
const CONFIG_FILE = join(import.meta.dirname, '../src/config/config.json')

async function main() {
  const config = fs.readJSONSync(CONFIG_FILE)
  const illustDir = join(config.download.path, 'data/artworks')
  fs.ensureDirSync(illustDir)

  const favDir = join(dataDir, 'tmp')
  const favList = await fs.readdir(favDir)
  let results = []
  for (const item of favList) {
    const json = fs.readJSONSync(join(favDir, item))
    results = json.illusts.concat(results)
    for (const illust of json.illusts) {
      // console.log('Illust.id: ', illust.id)
      fs.writeJSONSync(join(illustDir, `${illust.id}.json`), illust)
    }
  }

  const imgJsonPath = join(config.download.path, 'data/images.json')
  fs.ensureFileSync(imgJsonPath)
  updateImagesJson(results, imgJsonPath)
}

function updateImagesJson(illusts, imgJsonPath) {
  try {
    const results = []
    for (const json of illusts) {
      // console.log('Reading:', json.id)
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
        results.push(...json.meta_pages.map((e, i) => ({
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

    console.log('Results.length: ', results.length)
    console.log('Updating images json...')

    const oldJson = fs.readJSONSync(imgJsonPath, { throws: false }) || []
    const imagesJson = uniqBy(results.concat(oldJson), o => `${o.id}${o.part}`)
    fs.writeJSONSync(imgJsonPath, imagesJson)

    console.log('Update images json success.')
  } catch (err) {
    console.error(err.message)
  }
}

main()
