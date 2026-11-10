/**
 * Lot SILBATCH côté navigateur, version 2 : le même format que `crates/sil-batch`. Les octets
 * produits ici sont ceux que lit le nœud ; un écart casse la vérification du résultat.
 *
 * magic "SILBATCH" | version 2 | kind | format | reserved 0 | width u32 | height u32 | count u32
 * | options (16 octets) | count x (len u32 | octets), little-endian.
 */

/** Seul type de média encore proposé : les images (`image/batch`). */
export type BatchKind = 'image'

/** Options d'un lot image. Une option absente vaut 0. */
export interface BatchOptions {
  /** Qualité JPEG 1 à 100 ; 0 = défaut de l'encodeur. */
  quality: number
  /** 0 étirer, 1 tenir dans la boîte, 2 remplir la boîte et recadrer. */
  fit: number
  /** Quarts de tour : 0, 1 (90° à droite), 2 (180°), 3 (90° à gauche). */
  rotate: number
  /** Indicateurs de `FLAG_*`. */
  flags: number
  /** Réservés à zéro : `sil-batch` les refuse s'ils ne le sont pas. */
  bitrateKbps: number
  sampleRateHz: number
  channels: number
}

export interface BatchHeader {
  kind: BatchKind
  /** Format de sortie : code de `IMAGE_FORMATS`. */
  format: number
  /** Largeur et hauteur des images ; 0 = taille d'origine. */
  width: number
  height: number
  options: BatchOptions
}

export const FIT_STRETCH = 0
export const FIT_CONTAIN = 1
export const FIT_COVER = 2

export const FLAG_GRAYSCALE = 1
export const FLAG_FLIP_HORIZONTAL = 2
export const FLAG_FLIP_VERTICAL = 4

/** Plafonds du conteneur, comme `sil-batch` (MAX_RECORDS, MAX_RECORD_BYTES). */
export const MAX_RECORDS = 65_536
export const MAX_RECORD_BYTES = 8 * 1024 * 1024

export const IMAGE_FORMATS = [
  { code: 0, name: 'png' },
  { code: 1, name: 'jpeg' },
] as const

/** Extensions acceptées à l'envoi. */
export const BATCH_EXTENSIONS: Record<BatchKind, readonly string[]> = {
  image: ['png', 'jpg', 'jpeg'],
}

/** Le type de média d'un workload de lot ; undefined pour les autres workloads. */
export function batchKindOf(workload: string): BatchKind | undefined {
  if (workload === 'image/batch') return 'image'
  return undefined
}

export const NO_OPTIONS: BatchOptions = {
  quality: 0,
  fit: FIT_STRETCH,
  rotate: 0,
  flags: 0,
  bitrateKbps: 0,
  sampleRateHz: 0,
  channels: 0,
}

const HEADER_LEN = 24
const OPTIONS_LEN = 16
const VERSION = 2

/** Refuse une option hors de sa plage (miroir de `Options::validate`). */
export function checkOptions(kind: BatchKind, format: number, options: BatchOptions): string | undefined {
  if (!IMAGE_FORMATS.some((item) => item.code === format)) return 'format de sortie inconnu'
  if (kind !== 'image') return 'type de média inconnu'
  if (options.bitrateKbps !== 0 || options.sampleRateHz !== 0 || options.channels !== 0) {
    return 'option audio sur un lot d’images'
  }
  if (!Number.isInteger(options.quality) || options.quality < 0 || options.quality > 100) {
    return 'qualité JPEG hors de 0 à 100'
  }
  if (options.fit > FIT_COVER || options.fit < 0) return 'mode de redimensionnement inconnu'
  if (options.rotate < 0 || options.rotate > 3) return 'rotation hors de 0 à 3'
  if ((options.flags & ~(FLAG_GRAYSCALE | FLAG_FLIP_HORIZONTAL | FLAG_FLIP_VERTICAL)) !== 0) {
    return 'indicateur inconnu'
  }
  return undefined
}

/** Assemble un lot (version 2) à partir de son en-tête et de ses enregistrements. */
export function buildBatch(header: BatchHeader, records: readonly Uint8Array[]): Uint8Array<ArrayBuffer> {
  const problem = checkOptions(header.kind, header.format, header.options)
  if (problem) throw new Error(problem)
  if (records.length > MAX_RECORDS) throw new Error('trop de fichiers dans le lot')
  let size = HEADER_LEN + OPTIONS_LEN
  for (const record of records) {
    if (record.byteLength > MAX_RECORD_BYTES) throw new Error('fichier trop grand dans le lot')
    size += 4 + record.byteLength
  }
  const out = new Uint8Array(size)
  const view = new DataView(out.buffer)
  out.set(new TextEncoder().encode('SILBATCH'), 0)
  view.setUint8(8, VERSION)
  view.setUint8(9, 1)
  view.setUint8(10, header.format)
  view.setUint8(11, 0)
  view.setUint32(12, header.width, true)
  view.setUint32(16, header.height, true)
  view.setUint32(20, records.length, true)
  const options = header.options
  view.setUint8(24, options.quality)
  view.setUint8(25, options.fit)
  view.setUint8(26, options.rotate)
  view.setUint8(27, options.flags)
  view.setUint32(28, options.bitrateKbps, true)
  view.setUint32(32, options.sampleRateHz, true)
  view.setUint8(36, options.channels)
  // Octets 37 à 39 : réservés, à zéro (Uint8Array est déjà à zéro).
  let position = HEADER_LEN + OPTIONS_LEN
  for (const record of records) {
    view.setUint32(position, record.byteLength, true)
    position += 4
    out.set(record, position)
    position += record.byteLength
  }
  return out
}
