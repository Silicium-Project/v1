// Copie le moteur WASI compilé (cargo, cible wasm32-wasip1) dans public/engine/ pour le site.
// Usage : node scripts/copy-engine.mjs [chemin du .wasm]. Sans argument : target/wasm32-wasip1/release.
import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const source = resolve(process.argv[2] ?? join(here, '..', '..', '..', 'target', 'wasm32-wasip1', 'release', 'sil-raytracer-block.wasm'))
const target = join(here, '..', 'public', 'engine')
if (!existsSync(source)) {
  console.error(`moteur absent : ${source}\nlancer : cargo build -p sil-raytracer --bin sil-raytracer-block --target wasm32-wasip1 --release`)
  process.exit(1)
}
mkdirSync(target, { recursive: true })
copyFileSync(source, join(target, 'sil-raytracer-block.wasm'))
console.log(`copié : ${source} -> public/engine/sil-raytracer-block.wasm`)
