/** Base of every API path: empty in the browser (same origin), absolute in the desktop build. */
const API_BASE = (import.meta.env.VITE_API_BASE ?? '').replace(/\/$/, '')

/** A failed call. `code` is the API error code, or a local code: network, timeout, internal. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
  ) {
    super(code)
    this.name = 'ApiError'
  }
}

const TIMEOUT_MS = 15_000

interface RequestOptions {
  method?: string
  body?: unknown
  /** Raw bytes sent as application/octet-stream (content upload). */
  rawBody?: Uint8Array
  token?: string
  accept?: string
}

/**
 * Sends one request to the API (same origin). Credentials are never sent by the browser on
 * its own: the bearer token is the only authentication. Redirects are refused.
 */
export async function request(path: string, options: RequestOptions = {}): Promise<Response> {
  const headers: Record<string, string> = { Accept: options.accept ?? 'application/json' }
  let body: BodyInit | undefined
  if (options.rawBody !== undefined) {
    headers['Content-Type'] = 'application/octet-stream'
    // A Uint8Array is a valid body at run time; the DOM typings are stricter about its buffer type.
    body = options.rawBody as unknown as BodyInit
  } else if (options.body !== undefined) {
    headers['Content-Type'] = 'application/json'
    body = JSON.stringify(options.body)
  }
  if (options.token) headers.Authorization = `Bearer ${options.token}`

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), TIMEOUT_MS)
  let response: Response
  try {
    response = await fetch(`${API_BASE}${path}`, {
      method: options.method ?? 'GET',
      headers,
      body,
      signal: controller.signal,
      credentials: 'same-origin',
      cache: 'no-store',
      redirect: 'error',
    })
  } catch (failure) {
    const aborted = failure instanceof Error && failure.name === 'AbortError'
    throw new ApiError(0, aborted ? 'timeout' : 'network')
  } finally {
    clearTimeout(timer)
  }
  if (!response.ok) throw new ApiError(response.status, await errorCode(response))
  return response
}

/** Sends a request and reads a JSON answer. A 204 answer gives undefined. */
export async function requestJson<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const response = await request(path, options)
  if (response.status === 204) return undefined as T
  try {
    return (await response.json()) as T
  } catch {
    throw new ApiError(response.status, 'internal')
  }
}

/** Sends a request and reads the answer as a file (account export). */
export async function requestBlob(path: string, token: string): Promise<Blob> {
  const response = await request(path, { token, accept: 'application/json' })
  return response.blob()
}

async function errorCode(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { code?: unknown }
    return typeof body.code === 'string' ? body.code : 'internal'
  } catch {
    return 'internal'
  }
}
