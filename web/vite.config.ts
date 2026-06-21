import path from 'node:path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import Components from 'unplugin-vue-components/vite'
import AutoImport from 'unplugin-auto-import/vite'

export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
  base: './',
  server: {
    port: 3621,
  },
  // build: {
  //   copyPublicDir: false,
  // },
  plugins: [
    {
      name: 'inject-analytics',
      transformIndexHtml(html, { server }) {
        if (server) { return html }
        return {
          html,
          tags: [
            {
              tag: 'script',
              attrs: {
                'async': true,
                'defer': true,
                'data-website-id': 'ada7855c-2e95-4286-8b1c-240c67c79a94',
                'src': 'https://um.nanoka.top/script.js',
              },
              injectTo: 'head',
            },
          ],
        }
      },
    },
    vue(),
    AutoImport({
      imports: ['vue', 'vue-router'],
      dts: true,
      vueTemplate: true,
    }),
    Components({
      dts: true,
    }),
  ],
})
