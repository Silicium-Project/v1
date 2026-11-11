// Copie le runtime ONNX Runtime Web (CPU, wasm) dans public/ort/ : le navigateur le charge depuis le même
// site, sans hôte tiers (CSP 'self' avec 'wasm-unsafe-eval'). Version épinglée dans package.json.
// Usage : node scripts/copy-ort.mjs
import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const dist = join(here, '..', 'node_modules', 'onnxruntime-web', 'dist')
const target = join(here, '..', 'public', 'ort')
const FILES = ['ort-wasm-simd-threaded.mjs', 'ort-wasm-simd-threaded.wasm']

mkdirSync(target, { recursive: true })
for (const name of FILES) {
  const from = join(dist, name)
  if (!existsSync(from)) {
    console.error(`absent : ${from}\nLancer npm ci dans apps/web.`)
    process.exit(1)
  }
  copyFileSync(from, join(target, name))
}
console.log('runtime ONNX prêt : public/ort/')
