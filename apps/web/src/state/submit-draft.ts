import type { SceneModel } from '@/scene/types'

// Brouillon de la page de soumission (onglet courant) : un rafraîchissement ne doit pas remettre le
// workload ni la scène à leur valeur de départ. Effacé après une soumission réussie.
const STORAGE_KEY = 'silicium.submitDraft'

export interface SubmitDraft {
  workload: string
  label: string
  scene: SceneModel
}

function isScene(value: unknown): value is SceneModel {
  if (!value || typeof value !== 'object') return false
  const scene = value as Partial<SceneModel>
  return (
    typeof scene.width === 'number' &&
    typeof scene.height === 'number' &&
    typeof scene.fov === 'number' &&
    typeof scene.camera === 'object' &&
    scene.camera !== null &&
    Array.isArray(scene.objects) &&
    Array.isArray(scene.lights)
  )
}

/** Lit un brouillon brut : null si absent, illisible ou d'un workload inconnu. */
export function parseDraft(raw: string | null, workloads: readonly string[]): SubmitDraft | null {
  if (!raw) return null
  try {
    const value = JSON.parse(raw) as Partial<SubmitDraft>
    if (typeof value.workload !== 'string' || !workloads.includes(value.workload)) return null
    if (typeof value.label !== 'string' || !isScene(value.scene)) return null
    return { workload: value.workload, label: value.label, scene: value.scene }
  } catch {
    return null
  }
}

/** Brouillon enregistré dans l'onglet, ou null. */
export function loadDraft(workloads: readonly string[]): SubmitDraft | null {
  try {
    return parseDraft(sessionStorage.getItem(STORAGE_KEY), workloads)
  } catch {
    return null
  }
}

export function saveDraft(draft: SubmitDraft): void {
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(draft))
  } catch {
    // Storage refused: the draft lives in memory for this page only.
  }
}

export function clearDraft(): void {
  try {
    sessionStorage.removeItem(STORAGE_KEY)
  } catch {
    // Nothing to clear if storage is refused.
  }
}
