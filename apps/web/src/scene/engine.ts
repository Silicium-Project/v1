// Le moteur de rendu dans le navigateur : le même binaire que sur les nœuds (`sil-raytracer-block`,
// compilé en WebAssembly WASI), lancé par un shim WASI. Aperçus, vérification de scène et import
// .obj donnent donc exactement les octets et les diagnostics du moteur des nœuds.

import { ConsoleStdout, File, OpenFile, WASI } from '@bjorn3/browser_wasi_shim'

/** Où le site publie le moteur (copié à la construction, voir scripts/copy-engine.mjs). */
export const ENGINE_URL = `${import.meta.env.BASE_URL}engine/sil-raytracer-block.wasm`

export interface EngineIssue {
  severity: 'error' | 'warning'
  code: string
  line: number
  key?: string
  suggestion?: string
  message: string
}

/** Rapport de `sil-raytracer-block check` (le même que celui du moteur des nœuds). */
export interface EngineReport {
  valid: boolean
  errors: number
  warnings: number
  issues: EngineIssue[]
  scene: { width: number; height: number; spheres: number; planes: number; lights: number; triangles: number; samples: number; mode: string; engine: number }
}

let compiled: Promise<WebAssembly.Module> | undefined

function engineModule(): Promise<WebAssembly.Module> {
  compiled ??= fetch(ENGINE_URL).then(async (response) => {
    if (!response.ok) throw new Error(`moteur introuvable (${response.status})`)
    return WebAssembly.compile(await response.arrayBuffer())
  })
  return compiled
}

interface EngineRun {
  code: number
  stdout: Uint8Array
  stderr: string
}

/** Lance le moteur avec ces arguments ; l'entrée standard est `stdin`. */
export async function runEngine(args: string[], stdin: Uint8Array): Promise<EngineRun> {
  const input = new OpenFile(new File(stdin))
  const output = new OpenFile(new File(new Uint8Array(0)))
  let stderr = ''
  const error = new ConsoleStdout((bytes) => {
    stderr += new TextDecoder().decode(bytes)
  })
  const wasi = new WASI(['sil-raytracer-block', ...args], [], [input, output, error])
  const instance = await WebAssembly.instantiate(await engineModule(), {
    wasi_snapshot_preview1: wasi.wasiImport,
  })
  const code = wasi.start(instance as unknown as Parameters<WASI['start']>[0])
  return { code, stdout: output.file.data, stderr: stderr.trim() }
}

const encode = (text: string) => new TextEncoder().encode(text)

/** Pixels RGB24 des lignes `[y0, y1)` de la scène `text` (même résultat que sur un nœud). */
export async function renderRows(text: string, width: number, height: number, y0: number, y1: number): Promise<Uint8Array> {
  const run = await runEngine([String(width), String(height), String(y0), String(y1)], encode(text))
  if (run.code !== 0) throw new Error(run.stderr || `moteur : code ${run.code}`)
  return run.stdout
}

/** Diagnostics de la scène : mêmes règles que le moteur des nœuds. */
export async function checkScene(text: string): Promise<EngineReport> {
  const run = await runEngine(['check'], encode(text))
  if (run.code !== 0) throw new Error(run.stderr || `moteur : code ${run.code}`)
  return JSON.parse(new TextDecoder().decode(run.stdout)) as EngineReport
}

/** Bloc `mesh` d'un fichier .obj, centré et mis à l'échelle `size` (axe vertical `up`, y ou z). */
export async function importObj(text: string, material: string, size: number, up: 'y' | 'z'): Promise<string> {
  const run = await runEngine(['obj', material, String(size), up], encode(text))
  if (run.code !== 0) throw new Error(run.stderr || `moteur : code ${run.code}`)
  return new TextDecoder().decode(run.stdout)
}
