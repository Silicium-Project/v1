import { fileURLToPath, URL } from 'node:url'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'

// The API runs on 8080 in development; the Vite dev server forwards API calls so the
// browser only ever talks to one origin (same origin, no CORS).
const API = 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    proxy: {
      '/v1': API,
      '/health': API,
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: false,
    target: 'es2022',
    cssCodeSplit: true,
    assetsInlineLimit: 0,
  },
  test: {
    environment: 'happy-dom',
    include: ['src/**/*.test.ts'],
  },
})
