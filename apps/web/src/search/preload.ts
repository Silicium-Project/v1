/**
 * Chargement du modèle de recherche en arrière-plan : lancé à la soumission d'un job embedding (le job
 * met plusieurs minutes) et à l'ouverture d'un job embedding. Une seule fois : un chargement réussi reste
 * prêt tant que la page est ouverte, un échec peut être relancé.
 */
import { ref } from 'vue'
import { loadEmbedder } from './embedder'

export type PreloadState = 'idle' | 'loading' | 'ready' | 'failed'

export const preload = ref<{ state: PreloadState; percent: number | null }>({ state: 'idle', percent: null })

/** Lance le chargement s'il n'est ni en cours ni prêt. Sans effet pendant un chargement. */
export function startPreload(): void {
  if (preload.value.state === 'loading' || preload.value.state === 'ready') return
  preload.value = { state: 'loading', percent: null }
  loadEmbedder((loaded, total) => {
    preload.value = { state: 'loading', percent: total > 0 ? Math.round((loaded * 100) / total) : null }
  })
    .then(() => {
      preload.value = { state: 'ready', percent: 100 }
    })
    .catch(() => {
      preload.value = { state: 'failed', percent: null }
    })
}
