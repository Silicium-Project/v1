/** Raison pour laquelle un fichier ne donne pas de texte utilisable. */
export type DocumentErrorCode = 'unsupported' | 'invalid' | 'noText' | 'tooLarge'

export class DocumentError extends Error {
  constructor(readonly code: DocumentErrorCode) {
    super(code)
  }
}
