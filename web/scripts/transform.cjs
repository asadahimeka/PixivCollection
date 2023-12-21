const { join } = require('node:path')
const { readdir, readFile, writeFile } = require('node:fs/promises')

async function main() {
  try {
    const dirPath = 'E:\\Pictures\\Pixiv\\data\\artworks'
    const paths = await readdir(dirPath)
    const results = []
    for (const item of paths) {
      console.log('Reading:', item)
      const json = JSON.parse(await readFile(join(dirPath, item), 'utf-8'))
      if (json.meta_single_page.original_image_url) {
        results.push({
          id: json.id,
          part: 0,
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
      }
      else {
        results.push(...json.meta_pages.map((e, i) => ({
          id: json.id,
          part: i,
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
    console.log('results.length: ', results.length)
    await writeFile('./images.json', JSON.stringify(results.sort((a, b) => b.id - a.id)))
    // await writeFile('./images.f.json', JSON.stringify(results, null, 2))
  }
  catch (err) {
    console.error(err.message)
  }
}

main()
