/**
 * Texte d'un PDF, extrait dans le navigateur avec pdf.js (version épinglée dans package.json).
 * Le module est chargé à la demande : le moteur ne pèse que pour qui importe un PDF. Le fichier ne
 * quitte jamais l'appareil ; seul le texte extrait part vers le nœud, comme pour un fichier texte.
 */
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url'
import type * as PdfJs from 'pdfjs-dist'
import { DocumentError } from './errors'

/** Élément de texte d'une page, réduit aux champs lus (les marqueurs de contenu n'en ont pas). */
export interface PageItem {
  str?: string
  hasEOL?: boolean
}

/**
 * Lignes d'un ensemble de pages : le texte d'un élément s'ajoute à la ligne en cours, et `hasEOL`
 * la termine. Chaque page commence une ligne neuve. Les lignes vides et blanches sont retirées.
 */
export function linesOfPages(pages: PageItem[][]): string[] {
  return pagedLines(pages).map((line) => line.text)
}

/** Lignes avec le numéro de leur page (à partir de 1), dans l'ordre du document. */
export function pagedLines(pages: PageItem[][]): Array<{ text: string; page: number }> {
  const lines: Array<{ text: string; page: number }> = []
  pages.forEach((items, index) => {
    let line = ''
    for (const item of items) {
      line += item.str ?? ''
      if (item.hasEOL) {
        lines.push({ text: line, page: index + 1 })
        line = ''
      }
    }
    lines.push({ text: line, page: index + 1 })
  })
  // Un passage tient sur une ligne : un saut de ligne dans le texte d'un élément devient un espace.
  return lines
    .map((line) => ({ text: line.text.replace(/\s*[\r\n]+\s*/g, ' ').trim(), page: line.page }))
    .filter((line) => line.text !== '')
}

let loading: Promise<typeof PdfJs> | null = null

/** pdf.js et son worker, chargés une seule fois. */
function loadPdfJs(): Promise<typeof PdfJs> {
  loading ??= import('pdfjs-dist').then((pdfjs) => {
    pdfjs.GlobalWorkerOptions.workerSrc = workerUrl
    return pdfjs
  })
  return loading
}

/**
 * Lignes de texte d'un PDF, page après page. Un PDF illisible, protégé par mot de passe ou
 * corrompu lève `DocumentError('invalid')`. Un PDF sans couche de texte (scan) renvoie [].
 */
export async function pdfPassages(bytes: Uint8Array): Promise<Array<{ text: string; page: number }>> {
  const pdfjs = await loadPdfJs()
  // Copie : pdf.js détache le tampon qu'il reçoit, et le fichier source peut resservir.
  const task = pdfjs.getDocument({ data: bytes.slice(), disableFontFace: true, useSystemFonts: false })
  let document: PdfJs.PDFDocumentProxy
  try {
    document = await task.promise
  } catch {
    await task.destroy()
    throw new DocumentError('invalid')
  }
  try {
    const pages: PageItem[][] = []
    for (let number = 1; number <= document.numPages; number += 1) {
      const page = await document.getPage(number)
      const content = await page.getTextContent()
      pages.push(content.items.map((item) => ('str' in item ? { str: item.str, hasEOL: item.hasEOL } : {})))
      page.cleanup()
    }
    return pagedLines(pages)
  } catch {
    throw new DocumentError('invalid')
  } finally {
    await task.destroy()
  }
}
