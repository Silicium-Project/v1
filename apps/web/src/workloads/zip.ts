/**
 * Lecture d'une archive ZIP dans le navigateur, pour envoyer un lot d'un seul fichier `.zip`.
 * Seuls les contenus « stocké » (0) et « deflate » (8) sont acceptés, sans ZIP64 ni chiffrement.
 * Les dossiers, `__MACOSX` et les chemins qui sortent de l'archive sont ignorés.
 */

export interface ZipEntry {
  name: string
  data: Uint8Array
}

export class ZipError extends Error {}

const END_SIGNATURE = 0x06054b50
const CENTRAL_SIGNATURE = 0x02014b50
const LOCAL_SIGNATURE = 0x04034b50
const END_MIN_LEN = 22
/** Un commentaire d'archive tient dans 65 535 octets : la fin de répertoire est dans les derniers 65 557. */
const END_SEARCH_LEN = END_MIN_LEN + 0xffff

/**
 * Lit les fichiers d'une archive. `maxTotal` borne la taille totale décompressée : au-delà,
 * l'archive est refusée avant de produire davantage d'octets.
 */
export async function readZip(bytes: Uint8Array, maxTotal: number): Promise<ZipEntry[]> {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const end = findEndOfDirectory(view)
  const count = view.getUint16(end + 10, true)
  const directory = view.getUint32(end + 16, true)
  if (count === 0xffff || directory === 0xffffffff) throw new ZipError('ZIP64 non pris en charge')

  const entries: ZipEntry[] = []
  let total = 0
  let position = directory
  for (let index = 0; index < count; index += 1) {
    if (position + 46 > bytes.byteLength || view.getUint32(position, true) !== CENTRAL_SIGNATURE) {
      throw new ZipError('répertoire ZIP invalide')
    }
    const method = view.getUint16(position + 10, true)
    const compressedSize = view.getUint32(position + 20, true)
    const size = view.getUint32(position + 24, true)
    const nameLength = view.getUint16(position + 28, true)
    const extraLength = view.getUint16(position + 30, true)
    const commentLength = view.getUint16(position + 32, true)
    const localOffset = view.getUint32(position + 42, true)
    const name = new TextDecoder().decode(bytes.subarray(position + 46, position + 46 + nameLength))
    position += 46 + nameLength + extraLength + commentLength

    if (name.endsWith('/') || name.startsWith('__MACOSX/') || !isInsideArchive(name)) continue
    total += size
    if (total > maxTotal) throw new ZipError('archive trop volumineuse')
    const data = await readEntry(bytes, view, localOffset, method, compressedSize, size)
    entries.push({ name, data })
  }
  return entries.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }))
}

/** Position de l'enregistrement de fin de répertoire, cherché depuis la fin de l'archive. */
function findEndOfDirectory(view: DataView): number {
  const lowest = Math.max(0, view.byteLength - END_SEARCH_LEN)
  for (let at = view.byteLength - END_MIN_LEN; at >= lowest; at -= 1) {
    if (view.getUint32(at, true) === END_SIGNATURE) return at
  }
  throw new ZipError('archive ZIP invalide')
}

function isInsideArchive(name: string): boolean {
  return !name.startsWith('/') && !name.split('/').includes('..')
}

async function readEntry(
  bytes: Uint8Array,
  view: DataView,
  localOffset: number,
  method: number,
  compressedSize: number,
  size: number,
): Promise<Uint8Array> {
  if (localOffset + 30 > bytes.byteLength || view.getUint32(localOffset, true) !== LOCAL_SIGNATURE) {
    throw new ZipError('entrée ZIP invalide')
  }
  const start = localOffset + 30 + view.getUint16(localOffset + 26, true) + view.getUint16(localOffset + 28, true)
  const raw = bytes.subarray(start, start + compressedSize)
  if (raw.byteLength !== compressedSize) throw new ZipError('entrée ZIP tronquée')
  let data: Uint8Array
  if (method === 0) data = raw.slice()
  else if (method === 8) data = await inflate(raw, size)
  else throw new ZipError('méthode de compression non prise en charge')
  if (data.byteLength !== size) throw new ZipError('taille d’entrée ZIP incohérente')
  return data
}

/** Décompresse du deflate brut ; s'arrête dès que la taille annoncée est dépassée. */
async function inflate(raw: Uint8Array, size: number): Promise<Uint8Array> {
  const stream = new Blob([raw.slice()]).stream().pipeThrough(new DecompressionStream('deflate-raw'))
  const reader = stream.getReader()
  const out = new Uint8Array(size)
  let filled = 0
  for (;;) {
    const { done, value } = await reader.read()
    if (done) break
    if (filled + value.byteLength > size) {
      await reader.cancel()
      throw new ZipError('entrée ZIP plus grande que la taille annoncée')
    }
    out.set(value, filled)
    filled += value.byteLength
  }
  return out.subarray(0, filled)
}
