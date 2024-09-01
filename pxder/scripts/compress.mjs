import { argv } from 'node:process'
import { join } from 'node:path'
import { spawn } from 'node:child_process'
import fs from 'fs-extra'
import sharp from 'sharp'
import AdmZip from 'adm-zip'

const ffmpegExe = join(import.meta.dirname, '../ffmpeg.exe')
const CONFIG_FILE = join(import.meta.dirname, '../src/config/config.json')
let imgDirPath = ''
const imgErrList = []
const ugoiraErrList = []

async function main(type) {
  const config = await fs.readJSON(CONFIG_FILE)
  imgDirPath = join(config.download.path, '[bookmark] Public')
  await fs.ensureDir(join(config.download.path, 'bookmark_webp'))
  await fs.ensureDir(join(config.download.path, 'bookmark_ugoira'))

  const imgDir = await fs.readdir(imgDirPath)
  for (const item of imgDir) {
    if (/\.(jpg|jpeg|png)$/i.test(item)) {
      type != 'avif' ? compress(join(imgDirPath, item)) : compressToAvif(join(imgDirPath, item))
    }

    if (item.endsWith('.zip')) {
      await convertUgoira(join(imgDirPath, item))
    }
  }

  if (imgErrList.length) {
    await fs.writeJSON(join(imgDirPath, '../img_err_list.json'), imgErrList)
  }
  if (ugoiraErrList.length) {
    await fs.writeJSON(join(imgDirPath, '../ugoira_err_list.json'), ugoiraErrList)
  }
}

/**
 * @param {string} inputFilePath
 */
function compress(inputFilePath) {
  // 输入文件和输出文件路径
  // const inputFilePath = 'D:/Desktop/(121003304)水着カズサ.png'; // 输入图片路径
  // const outputFilePath = 'D:/Desktop/(121003304)水着カズサ.webp'; // 输出WebP图片路径
  const outputFilePath = join(imgDirPath, '../bookmark_webp', inputFilePath.split(/[\\/]/).pop().replace(/\.(jpg|jpeg|png)$/, '.webp'))

  if (fs.existsSync(outputFilePath)) return

  console.log('Compressing:', inputFilePath)

  // 使用 sharp 进行转换
  sharp(inputFilePath)
    .webp({ quality: 80 }) // 设置WebP的质量，可选参数
    .toFile(outputFilePath, (err, info) => {
      if (err) {
        console.error('Error converting image:', inputFilePath, err)
        imgErrList.push(inputFilePath)
      } else {
        console.log('Image converted successfully:', outputFilePath)
      }
    })
}

function compressToAvif(inputFilePath) {
  const outputFilePath = join(imgDirPath, '../bookmark_avif', inputFilePath.split(/[\\/]/).pop().replace(/\.(jpg|jpeg|png)$/, '.avif'))

  if (fs.existsSync(outputFilePath)) return

  console.log('Compressing:', inputFilePath)

  // 使用 sharp 进行转换
  sharp(inputFilePath)
    .toFormat('avif')
    .toFile(outputFilePath, (err, info) => {
      if (err) {
        console.error('Error converting image:', inputFilePath, err)
        imgErrList.push(inputFilePath)
      } else {
        console.log('Image converted successfully:', outputFilePath)
      }
    })
}

/**
 * @param {string} inputFilePath
 */
async function convertUgoira(inputFilePath) {
  try {
    const filename = inputFilePath.split(/[\\/]/).pop()
    const id = filename.match(/\((\d+)\)/)[1]
    const ugoiraDir = join(imgDirPath, '../bookmark_ugoira')
    const unzipDir = join(ugoiraDir, id)
    const outputFilePath = join(ugoiraDir, filename.replace('.zip', argv[3] == 'avif' ? '.avif' : '.mp4'))

    if (fs.existsSync(outputFilePath)) return

    console.log('Converting ugoira:', inputFilePath)

    // 加载 ZIP 文件
    const zip = new AdmZip(inputFilePath)
    // 解压到指定目录
    zip.extractAllTo(unzipDir, true)
    // await fs.copyFile(inputFilePath, join(ugoiraDir, filename))

    // eslint-disable-next-line no-undef
    const res = await fetch(`https://hibiapi.cocomi.eu.org/api/pixiv-web-api/illustUgoiraMeta?args=[${id}]`, {
      headers: { 'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36' }
    }).catch(() => null)

    const { input, rate } = await (async () => {
      let input
      if (res && res.ok) {
        const metadata = await res.json()
        if (metadata) {
          let ffconcat = 'ffconcat version 1.0\n'
          for (const item of metadata.frames) {
            ffconcat += 'file ' + item.file + '\n'
            ffconcat += 'duration ' + Number(item.delay) / 1000 + '\n'
          }
          // Fix ffmpeg concat demuxer issue. This will increase the frame count, but will fix the last frame timestamp issue.
          ffconcat += 'file ' + metadata.frames[metadata.frames.length - 1].file + '\n'
          input = join(unzipDir, 'ffconcat.txt')
          await fs.writeFile(input, ffconcat)
          return { input }
        }
      }
      input = join(unzipDir, '%06d.jpg')
      const len = (await fs.readdir(unzipDir)).length
      const totalMs = len * 40
      return {
        input: join(unzipDir, '%06d.jpg'),
        rate: len / totalMs * 1000
      }
    })()

    await runFFmpeg(input, outputFilePath, rate)

    await fs.remove(unzipDir)

    console.log('Convert ugoira success:', outputFilePath)
  } catch (err) {
    console.log(`Convert ugoira err ${inputFilePath}: ${err}`)
    ugoiraErrList.push(inputFilePath)
  }
}

function runFFmpeg(input, output, rate) {
  return new Promise((resolve, reject) => {
    const arg1 = rate ? ['-r', rate] : ['-f', 'concat']
    const arg2 = output.endsWith('.avif')
      ? ['-c:v', 'libsvtav1', '-pix_fmt', 'yuv420p', '-crf', '30', '-b:v', '0']
      : ['-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-vf', 'pad=ceil(iw/2)*2:ceil(ih/2)*2']
    const args = ['-y', ...arg1, '-i', input, ...arg2, output]

    console.log('ffmpeg', args.join(' '))
    // FFmpeg 命令和参数
    const ffmpeg = spawn(ffmpegExe, args)

    // 监听 stdout 数据
    // ffmpeg.stdout.on('data', (data) => {
    //   console.log(`ffmpeg stdout: ${data}`)
    // })

    // 监听 stderr 数据
    // ffmpeg.stderr.on('data', (data) => {
    //   console.error(`ffmpeg stderr: ${data}`)
    // })

    // 监听进程结束
    ffmpeg.on('close', (code) => {
      // console.log(`ffmpeg 子进程退出，退出码：${code}`);
      code == 0 ? resolve(code) : reject(code)
    })
  })
}

main(argv[2])
