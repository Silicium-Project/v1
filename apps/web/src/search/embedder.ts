/**
 * Vectorisation d'une question dans le navigateur, avec le même modèle que les passages (multilingual-e5-small,
 * fp32, ONNX Runtime 1.31.1). Le modèle et le tokenizer sont servis par le site, vérifiés par SHA-256
 * avant usage, et gardés dans le cache du navigateur. Chargés à la première question seulement.
 */
import { meanPoolNormalize, truncateIds } from './semantic'

export const MODEL_URL = '/download/models/multilingual-e5-small/model.onnx'
export const TOKENIZER_URL = '/download/models/multilingual-e5-small/tokenizer.json'
/** Empreintes épinglées : le même modèle que celui des nœuds (sil-wl-embed). */
export const MODEL_SHA256 = 'ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665'
export const TOKENIZER_SHA256 = '0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39'
const CACHE_NAME = 'silicium-models-v1'
/** Tokens spéciaux de XLM-R (multilingual-e5) : début et fin de séquence. */
const BOS = 0
const EOS = 2

export type Progress = (loaded: number, total: number) => void

export interface Embedder {
  /** Vecteur normalisé de 384 dimensions. `prefix` : QUERY_PREFIX pour une question, PASSAGE_PREFIX sinon. */
  embed(text: string, prefix: string): Promise<Float32Array>
}

async function sha256Hex(bytes: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')
}

/** Octets d'un fichier du site : cache du navigateur d'abord, sinon téléchargement avec progression. */
async function fetchVerified(url: string, expected: string, onProgress: Progress): Promise<ArrayBuffer> {
  const cache = await caches.open(CACHE_NAME).catch(() => null)
  const cached = await cache?.match(url)
  if (cached) {
    const bytes = await cached.arrayBuffer()
    if ((await sha256Hex(bytes)) === expected) {
      onProgress(bytes.byteLength, bytes.byteLength)
      return bytes
    }
    await cache?.delete(url)
  }
  const response = await fetch(url)
  if (!response.ok || !response.body) throw new Error(`fichier indisponible : ${response.status}`)
  const total = Number(response.headers.get('content-length')) || 0
  const reader = response.body.getReader()
  const chunks: Uint8Array[] = []
  let loaded = 0
  for (;;) {
    const { done, value } = await reader.read()
    if (done) break
    chunks.push(value)
    loaded += value.byteLength
    onProgress(loaded, total)
  }
  const bytes = await new Blob(chunks as BlobPart[]).arrayBuffer()
  if ((await sha256Hex(bytes)) !== expected) throw new Error('empreinte incorrecte')
  await cache?.put(url, new Response(bytes.slice(0))).catch(() => undefined)
  return bytes
}

let embedder: Promise<Embedder> | null = null

/** Charge le modèle une seule fois. Un échec permet de réessayer. */
export function loadEmbedder(onProgress: Progress): Promise<Embedder> {
  embedder ??= build(onProgress).catch((error: unknown) => {
    embedder = null
    throw error
  })
  return embedder
}

async function build(onProgress: Progress): Promise<Embedder> {
  // Build wasm seul (CPU) : pas de variante WebGPU à charger.
  const ort = await import('onnxruntime-web/wasm')
  ort.env.wasm.wasmPaths = '/ort/'
  ort.env.wasm.numThreads = 1
  const { Tokenizer } = await import('@huggingface/tokenizers')
  const tokenizerText = new TextDecoder().decode(await fetchVerified(TOKENIZER_URL, TOKENIZER_SHA256, onProgress))
  const tokenizer = new Tokenizer(JSON.parse(tokenizerText), {})
  const model = await fetchVerified(MODEL_URL, MODEL_SHA256, onProgress)
  const session = await ort.InferenceSession.create(model, { executionProviders: ['wasm'] })
  const output = session.outputNames[0]
  if (output === undefined) throw new Error('modèle sans sortie')

  return {
    async embed(text: string, prefix: string): Promise<Float32Array> {
      const ids = truncateIds(tokenizer.encode(prefix + text).ids, BOS, EOS)
      const length = ids.length
      const feeds = {
        input_ids: new ort.Tensor('int64', BigInt64Array.from(ids, (id) => BigInt(id)), [1, length]),
        attention_mask: new ort.Tensor('int64', new BigInt64Array(length).fill(1n), [1, length]),
        token_type_ids: new ort.Tensor('int64', new BigInt64Array(length), [1, length]),
      }
      const result = await session.run(feeds)
      const hidden = result[output]?.data
      if (!(hidden instanceof Float32Array)) throw new Error('sortie inattendue')
      return meanPoolNormalize(hidden, new Array<number>(length).fill(1))
    },
  }
}
