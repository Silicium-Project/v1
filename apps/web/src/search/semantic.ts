/**
 * Recherche sémantique sur les passages d'un job embedding. Pur calcul, sans navigateur : la
 * vectorisation de la question est dans `embedder.ts`. Les vecteurs du job sont déjà normalisés (L2),
 * donc le cosinus est un produit scalaire.
 */

/** Dimension des vecteurs de multilingual-e5-small. */
export const DIM = 384
/** Longueur maximale d'une entrée, en tokens (spéciaux compris), comme le nœud. */
export const MAX_TOKENS = 512
/** Préfixe des questions pour multilingual-e5 ; les passages ont « passage: » côté nœud. */
export const QUERY_PREFIX = 'query: '
/**
 * Score minimal pour qu'un passage soit affiché comme réponse. Mesuré avec multilingual-e5-small sur 4
 * passages : pertinents 0,85 à 0,90, hors sujet 0,70 à 0,74, et 0,81 pour un passage voisin. À recalibrer
 * sur les documents réels (un score sous ce seuil donne « Pas dans les documents »).
 */
export const MIN_SCORE = 0.83
/** Nombre de passages affichés au maximum. */
export const TOP_K = 5

export interface Passage {
  index: number
  text: string
  vector: number[]
}

export interface Hit {
  index: number
  text: string
  score: number
}

/**
 * Moyenne des tokens non masqués, puis normalisation L2. `hidden` : `tokens × DIM` en ordre de ligne ;
 * `mask` : 1 pour un token réel. Sans token réel, le vecteur est nul. Même calcul que le nœud.
 */
export function meanPoolNormalize(hidden: Float32Array, mask: number[]): Float32Array {
  const pooled = new Float32Array(DIM)
  let count = 0
  for (let row = 0; row < mask.length; row += 1) {
    if (mask[row] === 0) continue
    count += 1
    const start = row * DIM
    for (let dim = 0; dim < DIM; dim += 1) pooled[dim] = (pooled[dim] ?? 0) + (hidden[start + dim] ?? 0)
  }
  if (count > 0) {
    for (let dim = 0; dim < DIM; dim += 1) pooled[dim] = (pooled[dim] ?? 0) / count
  }
  let norm = 0
  for (let dim = 0; dim < DIM; dim += 1) norm += (pooled[dim] ?? 0) ** 2
  norm = Math.sqrt(norm)
  if (norm > 0) {
    for (let dim = 0; dim < DIM; dim += 1) pooled[dim] = (pooled[dim] ?? 0) / norm
  }
  return pooled
}

/**
 * Ids d'une entrée limitée à MAX_TOKENS. `ids` contient le début et la fin spéciaux (`bos` et `eos`) :
 * on coupe le contenu, pas les spéciaux, comme la troncature du tokenizer côté nœud.
 */
export function truncateIds(ids: number[], bos: number, eos: number): number[] {
  if (ids.length <= MAX_TOKENS) return ids
  return [bos, ...ids.slice(1, MAX_TOKENS - 1), eos]
}

/** Score d'un passage pour une question : cosinus, les deux vecteurs étant normalisés. */
export function score(query: Float32Array, vector: number[]): number {
  let total = 0
  for (let dim = 0; dim < DIM; dim += 1) total += (query[dim] ?? 0) * (vector[dim] ?? 0)
  return total
}

/**
 * Les passages les plus proches de la question, du meilleur au moins bon. Vide si aucun n'atteint
 * `minScore` : la réponse est alors « Pas dans les documents ».
 */
export function rank(query: Float32Array, passages: Passage[], minScore = MIN_SCORE, limit = TOP_K): Hit[] {
  return passages
    .map((passage) => ({ index: passage.index, text: passage.text, score: score(query, passage.vector) }))
    .filter((hit) => hit.score >= minScore)
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
}
