import path from 'node:path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import Components from 'unplugin-vue-components/vite'
import AutoImport from 'unplugin-auto-import/vite'

export default defineConfig(env => ({
  base: './',
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
  plugins: [
    vue(),
    AutoImport({
      imports: ['vue', 'vue-router'],
      dts: true,
      vueTemplate: true,
    }),
    Components({
      dts: true,
    }),
    {
      name: 'html-transform',
      transformIndexHtml(html) {
        if (env.command == 'serve') return html
        const buildDate = new Date().toLocaleString()
        return html
          .replace('</head>', '<script defer src="https://um.nanoka.top/script.js" data-website-id="69717e89-51a1-409a-80e6-89210a688efd"></script></head>')
          .replace('<body', `<body data-build-date="${buildDate}"`)
      },
    },
  ],
}))
