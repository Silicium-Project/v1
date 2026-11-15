// Copie le nœud et le moteur de rendu dans src-tauri/binaries/ sous le nom que Tauri attend
// (externalBin : <nom>-<triplet cible>.exe). L'installeur les place ensuite à côté de l'application,
// là où le shell les cherche (node_process.rs). Sources : dist/windows-amd64, construit depuis le
// dépôt (voir docs/technical/deployment.md). Usage : node scripts/prepare-sidecars.mjs
import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

// Sous Linux, la source est dist/linux-amd64 et les noms n'ont pas d'extension (voir
// src-tauri/tauri.linux.conf.json, qui remplace la cible Windows).
const LINUX = process.platform === 'linux'
const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..', '..', '..')
const source = join(root, 'dist', LINUX ? 'linux-amd64' : 'windows-amd64')
const target = join(here, '..', 'src-tauri', 'binaries')
const TRIPLE = LINUX ? 'x86_64-unknown-linux-gnu' : 'x86_64-pc-windows-msvc'
const EXT = LINUX ? '' : '.exe'
// Sous Linux, le lanceur d'isolation doit être à côté du nœud : sans lui, le nœud refuse de
// démarrer dès qu'un moteur de rendu est configuré (cli.rs : « lanceur sil-sandbox-launch introuvable »).
// Windows : le moteur de rendu est le raytracer en WebAssembly (ressource, voir plus bas), pas un exécutable.
const SIDECARS = LINUX ? ['silicium-node', 'sil-raytracer-block', 'sil-sandbox-launch'] : ['silicium-node']

mkdirSync(target, { recursive: true })
for (const name of SIDECARS) {
  const from = join(source, `${name}${EXT}`)
  if (!existsSync(from)) {
    console.error(`binaire absent : ${from}\nLe construire d'abord : cargo build --release -p sil-node --bin silicium-node -p sil-raytracer --bin sil-raytracer-block`)
    process.exit(1)
  }
  const to = join(target, `${name}-${TRIPLE}${EXT}`)
  copyFileSync(from, to)
  console.log(`sidecar prêt : src-tauri/binaries/${name}-${TRIPLE}${EXT}`)
}

if (!LINUX) {
  // Moteur de rendu Windows : WebAssembly (WASI), isolé par construction (crates/sil-render-wasm).
  // Ressource Tauri posée à côté du nœud : node_process.rs le cherche sous ce nom.
  const wasmFrom = join(source, 'sil-raytracer-block.wasm')
  if (!existsSync(wasmFrom)) {
    console.error(`moteur wasm absent : ${wasmFrom}\nLe construire : cargo build -p sil-raytracer --bin sil-raytracer-block --target wasm32-wasip1 --release`)
    process.exit(1)
  }
  const resources = join(here, '..', 'src-tauri', 'resources')
  mkdirSync(resources, { recursive: true })
  copyFileSync(wasmFrom, join(resources, 'sil-raytracer-block.wasm'))
  console.log('moteur de rendu prêt : src-tauri/resources/sil-raytracer-block.wasm')
}
