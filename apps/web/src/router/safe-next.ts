const DEFAULT_TARGET = '/calculs'

/**
 * Accepts only an in-app path after sign-in. Anything that could leave the site (absolute URLs,
 * protocol-relative //host, backslashes, control characters) falls back to the default page.
 */
export function safeNext(value: unknown): string {
  if (typeof value !== 'string') return DEFAULT_TARGET
  if (!value.startsWith('/') || value.startsWith('//')) return DEFAULT_TARGET
  if (value.includes('\\') || /[\u0000-\u001f\u007f]/.test(value)) return DEFAULT_TARGET
  return value
}
