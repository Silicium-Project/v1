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
const SIDECARS = ['silicium-node', 'sil-raytracer-block']

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
