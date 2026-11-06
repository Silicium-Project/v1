import { ref } from 'vue'
import type { Router } from 'vue-router'

export interface Session {
  token: string
  expiresAt: string
  email: string
}

const STORAGE_KEY = 'silicium.session'

/**
 * Reads a stored session. Anything malformed or expired gives null.
 * The session lives in sessionStorage: it ends with the tab. The strict Content-Security-Policy
 * of the API (script-src 'self') is what keeps the token out of reach of injected scripts.
 */
export function parseSession(raw: string | null, now: number): Session | null {
  if (!raw) return null
  try {
    const value = JSON.parse(raw) as Partial<Session>
    if (typeof value.token !== 'string' || typeof value.expiresAt !== 'string' || typeof value.email !== 'string') {
      return null
    }
    if (Date.parse(value.expiresAt) <= now) return null
    return { token: value.token, expiresAt: value.expiresAt, email: value.email }
  } catch {
    return null
  }
}

function readStored(): string | null {
  try {
    return sessionStorage.getItem(STORAGE_KEY)
  } catch {
    return null
  }
}

function writeStored(value: string | null): void {
  try {
    if (value === null) sessionStorage.removeItem(STORAGE_KEY)
    else sessionStorage.setItem(STORAGE_KEY, value)
  } catch {
    // Storage refused: the session lives in memory for this page only.
  }
}

const current = ref<Session | null>(parseSession(readStored(), Date.now()))

/** The session if it has not expired (pure: does not modify state). */
export function currentSession(): Session | null {
  const session = current.value
  if (session === null || Date.parse(session.expiresAt) <= Date.now()) return null
  return session
}

export function hasSession(): boolean {
  return currentSession() !== null
}

export function currentToken(): string | null {
  return currentSession()?.token ?? null
}

export function saveSession(session: Session): void {
  current.value = session
  writeStored(JSON.stringify(session))
}

export function clearSession(): void {
  current.value = null
  writeStored(null)
}

/** Ends the session and sends the user to sign in, with a notice that it expired. */
export async function endSession(router: Router): Promise<void> {
  clearSession()
  await router.push({ name: 'login', query: { expired: '1' } })
}
