/**
 * Passages d'un fichier envoyé au workload d'embedding : PDF, ZIP, TXT, MD. Tout est lu et extrait
 * dans le navigateur ; le nœud reçoit toujours un texte à une ligne par passage (voir embedding.ts).
 * Dans un ZIP, seuls les `.pdf`, `.txt` et `.md` sont lus ; les autres entrées sont ignorées.
 */
import { DocumentError } from './errors'
import { pdfLines } from './pdf'
import { readZip } from './zip'

export { DocumentError, type DocumentErrorCode } from './errors'

/** Taille maximale d'un fichier envoyé (avant extraction). */
export const MAX_UPLOAD_BYTES = 32 * 1024 * 1024
/** Taille maximale décompressée d'un ZIP : borne la mémoire, même pour une archive piégée. */
const MAX_ARCHIVE_BYTES = 32 * 1024 * 1024

/** Lignes d'un fichier texte UTF-8, vides retirées. */
export function textLines(bytes: Uint8Array): string[] {
  let text: string
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch {
    throw new DocumentError('invalid')
  }
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== '')
}

type Kind = 'pdf' | 'text' | 'zip'

function kindOf(name: string): Kind | null {
  const lower = name.toLowerCase()
  if (lower.endsWith('.pdf')) return 'pdf'
  if (lower.endsWith('.txt') || lower.endsWith('.md')) return 'text'
  if (lower.endsWith('.zip')) return 'zip'
  return null
}

/** Lignes des entrées PDF et texte d'une archive ZIP. */
async function zipLines(bytes: Uint8Array): Promise<string[]> {
  let entries: Awaited<ReturnType<typeof readZip>>
  try {
    entries = await readZip(bytes, MAX_ARCHIVE_BYTES)
  } catch {
    throw new DocumentError('invalid')
  }
  const lines: string[] = []
  let readable = 0
  for (const entry of entries) {
    const kind = kindOf(entry.name)
    // Un ZIP dans un ZIP n'est pas lu.
    if (kind === null || kind === 'zip') continue
    readable += 1
    const entryLines = kind === 'pdf' ? await pdfLines(entry.data) : textLines(entry.data)
    for (const line of entryLines) lines.push(line)
  }
  if (readable === 0) throw new DocumentError('unsupported')
  return lines
}

/**
 * Texte des passages d'un fichier, une ligne par passage, séparées par « \n ». Lève
 * `DocumentError` si le format n'est pas pris en charge, si le fichier est illisible, s'il est trop
 * grand, ou s'il ne contient aucun texte (un scan, par exemple).
 */
export async function passagesOf(file: File): Promise<string> {
  if (file.size > MAX_UPLOAD_BYTES) throw new DocumentError('tooLarge')
  const bytes = new Uint8Array(await file.arrayBuffer())
  const kind = kindOf(file.name)
  let lines: string[]
  if (kind === 'pdf') lines = await pdfLines(bytes)
  else if (kind === 'text') lines = textLines(bytes)
  else if (kind === 'zip') lines = await zipLines(bytes)
  else throw new DocumentError('unsupported')
  if (lines.length === 0) throw new DocumentError('noText')
  return lines.join('\n')
}
