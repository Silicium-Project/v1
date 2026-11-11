/**
 * Texte de tous les fichiers envoyés au workload d'embedding, quel que soit leur type. Chaque fichier
 * est lu en entier et extrait dans le navigateur ; le nœud reçoit `passages.txt`, une ligne par passage.
 * Un fichier sans texte (image, binaire, PDF scanné) fait échouer l'envoi en le nommant : rien n'est
 * ignoré en silence. Chaque passage garde son origine (fichier, page ou ligne), pour l'afficher ensuite.
 */
import { DocumentError, type DocumentErrorCode } from './errors'
import { pdfPassages } from './pdf'
import { readZip, type ZipEntry } from './zip'

export { DocumentError, type DocumentErrorCode } from './errors'

/** Taille maximale d'un fichier envoyé (avant extraction). */
export const MAX_UPLOAD_BYTES = 32 * 1024 * 1024
/** Taille maximale décompressée d'une archive : borne la mémoire, même pour une archive piégée. */
const MAX_ARCHIVE_BYTES = 128 * 1024 * 1024
/** Profondeur maximale d'archives dans des archives. */
const MAX_DEPTH = 3
/** Longueur maximale d'un libellé de job (nœud et API). */
const MAX_LABEL_RUNES = 64

interface Item {
  name: string
  bytes: Uint8Array
}

/** Un passage et l'endroit où il se trouve : « rapport.pdf · page 3 », « lot.zip › notes.txt · ligne 4 ». */
export interface Passage {
  text: string
  origin: string
}

/** Texte des passages (une ligne par passage) et leur origine, dans le même ordre. */
export interface Extraction {
  text: string
  origins: string[]
}

/** Libellé d'un job à partir des noms de fichiers : séparés par des virgules, 64 caractères au plus. */
export function labelOf(names: string[]): string {
  const runes = Array.from(names.join(', '))
  return runes.length <= MAX_LABEL_RUNES ? runes.join('') : `${runes.slice(0, MAX_LABEL_RUNES - 1).join('')}…`
}

/** Lignes non vides d'un texte, espaces de bord retirés (comme le nœud). */
export function textLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== '')
}

/** Passages d'un texte brut, avec leur numéro de ligne dans le fichier. */
function textPassages(name: string, text: string): Passage[] {
  const passages: Passage[] = []
  text.split(/\r?\n/).forEach((raw, index) => {
    const line = raw.trim()
    if (line !== '') passages.push({ text: line, origin: `${name} · ligne ${index + 1}` })
  })
  return passages
}

/** Passages numérotés par position (paragraphes, rangées) : « fichier · ligne N ». */
function numbered(name: string, lines: string[]): Passage[] {
  return lines.map((text, index) => ({ text, origin: `${name} · ligne ${index + 1}` }))
}

function startsWith(bytes: Uint8Array, prefix: string): boolean {
  if (bytes.byteLength < prefix.length) return false
  for (let at = 0; at < prefix.length; at += 1) {
    if (bytes[at] !== prefix.charCodeAt(at)) return false
  }
  return true
}

function decodeText(bytes: Uint8Array): string | null {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch {
    return null
  }
}

function xmlOf(bytes: Uint8Array): Document {
  const text = decodeText(bytes)
  if (text === null) throw new DocumentError('invalid')
  const document = new DOMParser().parseFromString(text, 'text/xml')
  if (document.getElementsByTagName('parsererror').length > 0) throw new DocumentError('invalid')
  return document
}

/** Nom local d'un élément, sans préfixe (w:p, a:p et p donnent p). */
function localOf(element: Element): string {
  return (element.localName || element.tagName).split(':').pop() ?? ''
}

/** Éléments de ce nom local (ignore le préfixe : w:p, a:p, p). */
function byLocalName(root: ParentNode, name: string): Element[] {
  return Array.from(root.querySelectorAll('*')).filter((element) => localOf(element) === name)
}

/** Un paragraphe Word ou PowerPoint par ligne. */
function paragraphsOf(bytes: Uint8Array): string[] {
  return textLines(byLocalName(xmlOf(bytes), 'p').map((p) => p.textContent ?? '').join('\n'))
}

/** Classeur Excel : une ligne par rangée, cellules séparées par des tabulations. */
function sheetLines(bytes: Uint8Array, shared: string[]): string[] {
  const lines: string[] = []
  for (const row of byLocalName(xmlOf(bytes), 'row')) {
    const cells: string[] = []
    for (const cell of Array.from(row.children).filter((child) => localOf(child) === 'c')) {
      const type = cell.getAttribute('t')
      const value = cell.getElementsByTagName('v')[0]?.textContent ?? ''
      if (type === 's') cells.push(shared[Number(value)] ?? '')
      else if (type === 'inlineStr') cells.push(cell.textContent ?? '')
      else cells.push(value)
    }
    const line = cells.filter((cell) => cell !== '').join('\t')
    if (line !== '') lines.push(line)
  }
  return lines
}

/** Texte d'un document Office (docx, pptx, xlsx) : ce sont des archives ZIP de fichiers XML. */
async function officePassages(name: string, bytes: Uint8Array): Promise<Passage[]> {
  const entries = await zipEntries(name, bytes)
  const byName = new Map(entries.map((entry): [string, Uint8Array] => [entry.name, entry.data]))
  const xmlNames = [...byName.keys()].filter((entry) => entry.endsWith('.xml')).sort(compareNames)
  const shared = byName.has('xl/sharedStrings.xml')
    ? byLocalName(xmlOf(byName.get('xl/sharedStrings.xml') ?? new Uint8Array()), 'si').map((si) => si.textContent ?? '')
    : []
  const passages: Passage[] = []
  for (const entry of xmlNames) {
    const data = byName.get(entry) ?? new Uint8Array()
    const part = `${name} › ${entry}`
    if (entry.startsWith('xl/worksheets/')) passages.push(...numbered(part, sheetLines(data, shared)))
    else if (entry.startsWith('word/') || entry.startsWith('ppt/')) passages.push(...numbered(part, paragraphsOf(data)))
  }
  return passages
}

function compareNames(a: string, b: string): number {
  return a.localeCompare(b, undefined, { numeric: true })
}

/** Texte d'une page HTML, avec un saut de ligne après chaque bloc. */
function htmlPassages(name: string, bytes: Uint8Array): Passage[] {
  const text = decodeText(bytes)
  if (text === null) throw new DocumentError('invalid')
  const document = new DOMParser().parseFromString(text, 'text/html')
  document.querySelectorAll('script, style').forEach((element) => element.remove())
  document.querySelectorAll('p, div, li, tr, br, h1, h2, h3, h4, h5, h6').forEach((element) => {
    element.after('\n')
  })
  return numbered(name, textLines(document.body?.textContent ?? ''))
}

async function zipEntries(name: string, bytes: Uint8Array): Promise<ZipEntry[]> {
  try {
    return await readZip(bytes, MAX_ARCHIVE_BYTES)
  } catch {
    throw new DocumentError('invalid', [name])
  }
}

/**
 * Passages d'un fichier, selon son type. Un type inconnu qui n'est pas du texte lève `unsupported` : il
 * n'est jamais ignoré en silence.
 */
async function extractPassages(name: string, bytes: Uint8Array, depth: number): Promise<Passage[]> {
  if (depth > MAX_DEPTH) throw new DocumentError('invalid', [name])
  const lower = name.toLowerCase()
  if (lower.endsWith('.pdf') || startsWith(bytes, '%PDF-')) return pdfOf(name, bytes)
  if (lower.endsWith('.docx') || lower.endsWith('.pptx') || lower.endsWith('.xlsx')) {
    return named(name, () => officePassages(name, bytes))
  }
  if (lower.endsWith('.zip') || startsWith(bytes, 'PK\u0003\u0004')) return zipOf(name, bytes, depth)
  if (lower.endsWith('.html') || lower.endsWith('.htm')) return htmlPassages(name, bytes)
  const text = decodeText(bytes)
  if (text === null) throw new DocumentError('unsupported', [name])
  return textPassages(name, text)
}

async function pdfOf(name: string, bytes: Uint8Array): Promise<Passage[]> {
  const pages = await named(name, () => pdfPassages(bytes))
  if (pages.length === 0) throw new DocumentError('noText', [name])
  return pages.map((page) => ({ text: page.text, origin: `${name} · page ${page.page}` }))
}

/** Ajoute le nom du fichier à une erreur qui ne le porte pas encore. */
async function named<T>(name: string, work: () => Promise<T>): Promise<T> {
  try {
    return await work()
  } catch (error) {
    if (error instanceof DocumentError && error.names.length === 0) {
      throw new DocumentError(error.code, [name])
    }
    throw error
  }
}

/**
 * Passages de tous les éléments. Les éléments en échec sont rassemblés et signalés ensemble, pour que
 * l'utilisateur sache exactement lesquels retirer.
 */
async function eachItem(items: Item[], depth: number): Promise<Passage[]> {
  const passages: Passage[] = []
  const failed: string[] = []
  let code: DocumentErrorCode | null = null
  for (const item of items) {
    try {
      for (const passage of await extractPassages(item.name, item.bytes, depth)) passages.push(passage)
    } catch (error) {
      if (!(error instanceof DocumentError)) throw error
      failed.push(...(error.names.length > 0 ? error.names : [item.name]))
      code ??= error.code
    }
  }
  if (failed.length > 0) throw new DocumentError(code ?? 'unsupported', failed)
  return passages
}

/** Entrées d'une archive ZIP, de n'importe quel type, lues en entier. Leur origine garde le nom de l'archive. */
async function zipOf(name: string, bytes: Uint8Array, depth: number): Promise<Passage[]> {
  const items = (await zipEntries(name, bytes)).map((entry) => ({ name: `${name} › ${entry.name}`, bytes: entry.data }))
  return eachItem(items, depth + 1)
}

/**
 * Texte des passages de tous les fichiers, une ligne par passage, et leur origine, dans le même ordre.
 * Lève `DocumentError` avec la liste des fichiers concernés si l'un d'eux ne donne pas de texte.
 */
export async function passagesWithOrigins(files: File[]): Promise<Extraction> {
  const tooLarge = files.filter((file) => file.size > MAX_UPLOAD_BYTES).map((file) => file.name)
  if (tooLarge.length > 0) throw new DocumentError('tooLarge', tooLarge)
  const items: Item[] = []
  for (const file of files) {
    items.push({ name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) })
  }
  const passages = await eachItem(items, 0)
  if (passages.length === 0) throw new DocumentError('noText')
  return {
    text: passages.map((passage) => passage.text).join('\n'),
    origins: passages.map((passage) => passage.origin),
  }
}

/** Texte des passages seul, une ligne par passage. */
export async function passagesOf(files: File[]): Promise<string> {
  return (await passagesWithOrigins(files)).text
}
