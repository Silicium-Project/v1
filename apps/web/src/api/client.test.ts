import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError, requestJson } from './client'
import { errorMessageKey, isSessionEnded } from './errors'

afterEach(() => vi.unstubAllGlobals())

function stubFetch(impl: (url: string, init: RequestInit) => Promise<Response>) {
  const spy = vi.fn(impl)
  vi.stubGlobal('fetch', spy)
  return spy
}

describe('requestJson', () => {
  it('sends the bearer token and a JSON body', async () => {
    const spy = stubFetch(async () => new Response('{"ok":true}', { status: 200 }))
    await requestJson('/v1/x', { method: 'POST', token: 'sils_abc', body: { a: 1 } })
    const init = spy.mock.calls[0][1]
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer sils_abc')
    expect(init.body).toBe('{"a":1}')
    expect(init.redirect).toBe('error')
    expect(init.credentials).toBe('same-origin')
  })

  it('turns the API error code into an ApiError', async () => {
    stubFetch(async () => new Response('{"code":"email_taken","message":"server text"}', { status: 409 }))
    const error = await requestJson('/v1/auth/register').catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect(error).toMatchObject({ status: 409, code: 'email_taken' })
  })

  it('reports a network failure with its own code', async () => {
    stubFetch(async () => {
      throw new TypeError('Failed to fetch')
    })
    await expect(requestJson('/v1/x')).rejects.toMatchObject({ code: 'network', status: 0 })
  })

  it('reports a timeout when the request is aborted', async () => {
    stubFetch(async () => {
      throw Object.assign(new Error('aborted'), { name: 'AbortError' })
    })
    await expect(requestJson('/v1/x')).rejects.toMatchObject({ code: 'timeout' })
  })

  it('gives undefined for a 204 answer', async () => {
    stubFetch(async () => new Response(null, { status: 204 }))
    await expect(requestJson('/v1/x', { method: 'DELETE' })).resolves.toBeUndefined()
  })

  it('treats a non-JSON error body as an internal error', async () => {
    stubFetch(async () => new Response('<html>502</html>', { status: 502 }))
    await expect(requestJson('/v1/x')).rejects.toMatchObject({ status: 502, code: 'internal' })
  })
})

describe('errorMessageKey', () => {
  it('maps a known code to its message key', () => {
    expect(errorMessageKey(new ApiError(429, 'rate_limited'))).toBe('errors.rate_limited')
  })

  it('never shows an unknown code or a plain error text', () => {
    expect(errorMessageKey(new ApiError(418, 'teapot'))).toBe('errors.generic')
    expect(errorMessageKey(new Error('stack trace with secrets'))).toBe('errors.generic')
  })
})

describe('isSessionEnded', () => {
  it('is true only for a refused token', () => {
    expect(isSessionEnded(new ApiError(401, 'unauthenticated'))).toBe(true)
    expect(isSessionEnded(new ApiError(403, 'forbidden'))).toBe(false)
    expect(isSessionEnded(new Error('x'))).toBe(false)
  })
})
