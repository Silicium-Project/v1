import { ref } from 'vue'
import { en } from './en'
import { fr, type Messages } from './fr'

export type Locale = 'fr' | 'en'
export const LOCALES: readonly Locale[] = ['fr', 'en']

const STORAGE_KEY = 'silicium.locale'
const catalogs: Record<Locale, Messages> = { fr, en }

export const locale = ref<Locale>('fr')

/** Picks the locale: an explicit stored choice wins, then the browser languages, then French. */
export function detectLocale(stored: string | null, languages: readonly string[]): Locale {
  if (stored === 'fr' || stored === 'en') return stored
  for (const language of languages) {
    const base = language.toLowerCase().split('-')[0]
    if (base === 'fr' || base === 'en') return base
  }
  return 'fr'
}

export function setLocale(next: Locale): void {
  locale.value = next
  document.documentElement.lang = next
  try {
    localStorage.setItem(STORAGE_KEY, next)
  } catch {
    // Storage refused: the choice lasts for this page only.
  }
}

export function initLocale(): void {
  let stored: string | null = null
  try {
    stored = localStorage.getItem(STORAGE_KEY)
  } catch {
    stored = null
  }
  setLocale(detectLocale(stored, navigator.languages ?? [navigator.language]))
}

/** Translates a dotted key. Falls back to French, then to the key itself. */
export function t(path: string, params: Record<string, string | number> = {}): string {
  const found = lookup(catalogs[locale.value], path) ?? lookup(fr, path)
  if (found === undefined) {
    if (import.meta.env.DEV) console.warn(`missing message: ${path}`)
    return path
  }
  return found.replace(/\{(\w+)\}/g, (match: string, name: string) =>
    name in params ? String(params[name]) : match,
  )
}

/** The whole catalogue of the current locale (for long-form pages). */
export function messages(): Messages {
  return catalogs[locale.value]
}

function lookup(source: unknown, path: string): string | undefined {
  let node: unknown = source
  for (const part of path.split('.')) {
    if (node === null || typeof node !== 'object') return undefined
    node = (node as Record<string, unknown>)[part]
  }
  return typeof node === 'string' ? node : undefined
}
