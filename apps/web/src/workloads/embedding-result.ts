/**
 * Résultat du workload `embedding/e5-small` rendu lisible : un passage et son vecteur par ligne.
 * Le résultat est un vecteur de 384 f32 little-endian par passage, dans l'ordre du fichier envoyé.
 */

export const VECTOR_DIM = 384
const VECTOR_BYTES = VECTOR_DIM * 4

export interface PassageRow {
  index: number
  text: string
  vector: number[]
}

/** Passages d'un fichier d'entrée : une ligne non vide par passage, comme le nœud. */
export function passagesOf(input: string): string[] {
  return input
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== '')
}

/**
 * Octets bruts du résultat, depuis le fichier texte que l'API donne pour un job : une ligne
 * `fragment N: <hex>` par fragment. Les fragments se concatènent dans leur ordre.
 */
export function resultBytesOf(text: string): Uint8Array {
  const parts: Array<{ fragment: number; hex: string }> = []
  for (const line of text.split('\n')) {
    const match = /^fragment (\d+): ([0-9a-f]*)\s*$/.exec(line)
    if (match) parts.push({ fragment: Number(match[1]), hex: match[2] ?? '' })
  }
  if (parts.length === 0) throw new Error('résultat sans fragment')
  parts.sort((a, b) => a.fragment - b.fragment)
  const hex = parts.map((part) => part.hex).join('')
  if (hex.length % 2 !== 0) throw new Error('résultat hexadécimal incomplet')
  const bytes = new Uint8Array(hex.length / 2)
  for (let at = 0; at < bytes.length; at += 1) {
    bytes[at] = Number.parseInt(hex.slice(at * 2, at * 2 + 2), 16)
  }
  return bytes
}

/** Décode le résultat : un vecteur par passage. Lève une erreur si la taille ne tombe pas juste. */
export function decodeVectors(bytes: Uint8Array): number[][] {
  if (bytes.byteLength % VECTOR_BYTES !== 0) throw new Error('résultat de taille inattendue')
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const vectors: number[][] = []
  for (let offset = 0; offset < bytes.byteLength; offset += VECTOR_BYTES) {
    const vector: number[] = []
    for (let dim = 0; dim < VECTOR_DIM; dim += 1) {
      vector.push(view.getFloat32(offset + dim * 4, true))
    }
    vectors.push(vector)
  }
  return vectors
}

/** Associe chaque passage à son vecteur. Refuse un résultat qui ne correspond pas à l'entrée. */
export function passageRows(passages: string[], vectors: number[][]): PassageRow[] {
  if (passages.length !== vectors.length) {
    throw new Error('passages et vecteurs ne correspondent pas')
  }
  return passages.map((text, index) => ({ index, text, vector: vectors[index] ?? [] }))
}

export function toJson(rows: PassageRow[]): string {
  return JSON.stringify(rows.map(({ index, text, vector }) => ({ index, text, vector })))
}

/**
 * CSV : index, texte, puis les 384 valeurs. Un texte qui commence par `= + - @` est préfixé d'une
 * apostrophe, pour qu'un tableur ne l'exécute pas comme une formule.
 */
export function toCsv(rows: PassageRow[]): string {
  const header = ['index', 'text', ...Array.from({ length: VECTOR_DIM }, (_, dim) => `v${dim}`)]
  const lines = [header.join(',')]
  for (const row of rows) {
    lines.push([String(row.index), csvText(row.text), ...row.vector.map(String)].join(','))
  }
  return lines.join('\n') + '\n'
}

function csvText(text: string): string {
  const safe = /^[=+\-@]/.test(text) ? `'${text}` : text
  return `"${safe.replace(/"/g, '""')}"`
}
