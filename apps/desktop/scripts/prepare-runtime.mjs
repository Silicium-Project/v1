// Copie la bibliothèque ONNX Runtime 1.31.1 (version mesurée pour l'embedding) dans
// src-tauri/resources/, pour qu'elle soit livrée à côté du nœud. Source : dist/<plateforme>.
// Le nœud la cherche à côté de son binaire, ou dans ../lib/Silicium sous Linux.
import { copyFileSync, existsSync, mkdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const LINUX = process.platform === 'linux'
const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..', '..', '..')
const name = LINUX ? 'libonnxruntime.so' : 'onnxruntime.dll'
const from = join(root, 'dist', LINUX ? 'linux-amd64' : 'windows-amd64', name)
const resources = join(here, '..', 'src-tauri', 'resources')

if (!existsSync(from)) {
  console.error(`runtime absent : ${from}\nONNX Runtime 1.31.1 requis (voir docs/technical/embedding-workload.md).`)
  process.exit(1)
}
mkdirSync(resources, { recursive: true })
copyFileSync(from, join(resources, name))
console.log(`runtime prêt : src-tauri/resources/${name}`)
