<h1 align="center"> ${{\color{#0398fa}Pixiv}Collection{\color{#39c5bb}}}$ </h1>

![](https://upload-bbs.miyoushe.com/upload/2025/07/01/190122060/f27220d08df0cd8aa3f14968cc4eca58_9107049624435993727.webp)

## 简介

[示例站点](https://pxcl.cocomi.eu.org/)

将个人的 Pixiv 收藏夹数据爬取到本地，并部署为在线网站

无后端设计，图片数据一次性全部加载，图片较多时可能需要较长的时间

## 功能

- 图片爬取
  - 设置好用户 ID 后一键爬取公开收藏夹作品并存储 JSON 数据到本地
- 图片浏览
  - 瀑布流布局，可自定义瀑布流列数与间隔
  - 简易的图片浏览器，支持PC端和移动端的图片缩放与拖动
- 图片筛选
  - 通过发布年份、形状、尺寸、不健全度、R18、作者、标签、收藏数筛选图片
- 图片搜索
  - 通过图片id、图片标题、作者id、作者昵称、标签、标签翻译搜索图片
- 夜间模式
- 全屏模式

## 开发

> [!NOTE]
> 构建前端需要安装 Node.js >= 20.16.0 环境及 yarn v1 包管理器

```bash
# 安装依赖
yarn install

# 启动服务
yarn dev
```

## 部署

```bash
# 安装依赖
yarn install

# 构建应用
yarn build
```

## Credits

- [PixivCollection](https://github.com/orilights/PixivCollection)：原项目，修改于此

## Disclaimer

本项目与 pixiv.net(ピクシブ株式会社) 无任何隶属关系。

本项目网站所展示的所有作品的版权均为 Pixiv 或其原作者所有。

本项目仅供交流与学习，不得用于任何商业用途。

## License

Licensed under the [MIT](https://github.com/asadahimeka/PixivCollection/blob/tauri/LICENSE) license

Copyright © 2024 Yumine Sakura

<p><img src="https://count.nanoka.top/@himekapxclweb" alt="PixivCollection"></p>
