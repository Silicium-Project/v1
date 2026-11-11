/** Raison pour laquelle un fichier ne donne pas de texte utilisable. */
export type DocumentErrorCode = 'unsupported' | 'invalid' | 'noText' | 'tooLarge'

export class DocumentError extends Error {
  /** Noms des fichiers concernés (pour `unsupported`, `invalid` et `tooLarge`). */
  readonly names: string[]

  constructor(readonly code: DocumentErrorCode, names: string[] = []) {
    super(code)
    this.names = names
  }
}
