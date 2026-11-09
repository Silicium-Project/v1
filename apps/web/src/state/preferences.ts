// Préférences de cet appareil (localStorage) : elles ne quittent pas le navigateur et ne sont pas
// synchronisées avec le compte, que l'API ne stocke pas.

const STORAGE_KEY = 'silicium.preferences'

export interface Preferences {
  /** Type de calcul présélectionné à la soumission ; null : le premier de la liste. */
  defaultWorkload: string | null
}

export const DEFAULT_PREFERENCES: Preferences = { defaultWorkload: null }

/** Lit des préférences brutes : les valeurs invalides retombent sur le défaut. */
export function parsePreferences(raw: string | null, workloads: readonly string[]): Preferences {
  if (!raw) return { ...DEFAULT_PREFERENCES }
  try {
    const value = JSON.parse(raw) as Partial<Preferences>
    const known = typeof value.defaultWorkload === 'string' && workloads.includes(value.defaultWorkload)
    return { defaultWorkload: known ? value.defaultWorkload! : null }
  } catch {
    return { ...DEFAULT_PREFERENCES }
  }
}

export function loadPreferences(workloads: readonly string[]): Preferences {
  try {
    return parsePreferences(localStorage.getItem(STORAGE_KEY), workloads)
  } catch {
    return { ...DEFAULT_PREFERENCES }
  }
}

export function savePreferences(preferences: Preferences): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences))
  } catch {
    // Storage refused: the choice lasts for this page only.
  }
}
