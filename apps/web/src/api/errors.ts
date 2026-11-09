import { ApiError } from './client'

/** Every code the API or the client can produce. Each one has a message in i18n (errors.*). */
export const KNOWN_ERROR_CODES = [
  'network',
  'timeout',
  'unauthenticated',
  'invalid_credentials',
  'email_taken',
  'wrong_password',
  'not_an_image',
  'invalid_input',
  'rate_limited',
  'node_unavailable',
  'node_rejected',
  'not_a_job',
  'not_found',
  'too_large',
  'invalid_proof',
  'bad_link_code',
  'key_in_use',
  'too_many_devices',
  'job_taken',
  'too_many_jobs',
  'registration_closed',
  'internal',
  'busy',
] as const

/** The API code of a failed call; '' for anything that is not an API failure. */
export function errorCodeOf(error: unknown): string {
  return error instanceof ApiError ? error.code : ''
}

/** Translation key for an error. Unknown codes fall back to a generic message, never to server text. */
export function errorMessageKey(error: unknown): string {
  const code = errorCodeOf(error)
  return (KNOWN_ERROR_CODES as readonly string[]).includes(code) ? `errors.${code}` : 'errors.generic'
}

/** True when the token is no longer accepted: the user must sign in again. */
export function isSessionEnded(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401
}
