/**
 * Entrée du workload `embedding/e5-small` : un texte par ligne, en UTF-8. Le nœud ignore les lignes
 * vides (sil-wl-embed, `lines_of`) : le site fait de même pour compter et pour refuser un texte vide.
 */

export const PASSAGES_FILE_NAME = 'passages.txt'

export interface PassagesCheck {
  /** Le fichier à envoyer, ou null si le texte n'est pas acceptable. */
  file: File | null
  /** Nombre de passages (lignes non vides). */
  count: number
  /** Taille du fichier en octets (UTF-8). */
  bytes: number
  /** Raison du refus, vide si le texte est acceptable. */
  error: 'empty' | 'tooLarge' | ''
}

/** Vérifie un texte de passages et prépare le fichier envoyé au nœud. */
export function passagesFile(text: string, maxBytes: number): PassagesCheck {
  const passages = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== '')
  const body = passages.join('\n')
  const bytes = new TextEncoder().encode(body).length
  if (passages.length === 0) return { file: null, count: 0, bytes: 0, error: 'empty' }
  if (bytes > maxBytes) return { file: null, count: passages.length, bytes, error: 'tooLarge' }
  const file = new File([body], PASSAGES_FILE_NAME, { type: 'text/plain;charset=utf-8' })
  return { file, count: passages.length, bytes, error: '' }
}
