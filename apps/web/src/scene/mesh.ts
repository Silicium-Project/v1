// Lecture des maillages d'un bloc `mesh = { ... vertices = { ... } faces = { ... } }` (tel que le moteur
// l'écrit à l'import d'un .obj). Seuls les nombres comptent : le bloc est reconstruit par cfg.ts.

import type { Model } from './models'

/** Les sommets et les faces d'un bloc `mesh` ; une erreur si le bloc n'en contient pas. */
export function meshFromBlock(block: string): Model {
  const vertices = listOf(block, 'vertices')
  const faces = listOf(block, 'faces')
  if (vertices.length === 0 || faces.length === 0) throw new Error('le fichier ne contient aucune face')
  return { vertices, faces }
}

/** Les nombres de la liste `key = { ... }` du bloc, dans l'ordre. */
function listOf(block: string, key: string): number[] {
  const match = new RegExp(`${key}\\s*=\\s*\\{([^}]*)\\}`).exec(block)
  if (!match) return []
  return (match[1] ?? '')
    .split(/\s+/)
    .filter((token) => token !== '')
    .map(Number)
    .filter((value) => Number.isFinite(value))
}
