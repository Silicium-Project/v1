import { fileURLToPath, URL } from 'node:url'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// The desktop application (worker dashboard and machine linking). It is a separate bundle from
// the public site: its entry is src/desktop/index.html, and it writes dist-desktop/, which the
// Tauri shell loads (apps/desktop/src-tauri/tauri.conf.json). Mode `desktop` loads .env.desktop.
const webRoot = fileURLToPath(new URL('.', import.meta.url))

export default defineConfig({
  root: fileURLToPath(new URL('./src/desktop', import.meta.url)),
  envDir: webRoot,
  plugins: [vue()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: fileURLToPath(new URL('./dist-desktop', import.meta.url)),
    emptyOutDir: true,
    sourcemap: false,
    target: 'es2022',
    assetsInlineLimit: 0,
  },
})
