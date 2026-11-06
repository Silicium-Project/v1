import { describe, expect, it } from 'vitest'
import { parseSession } from './session'

const NOW = Date.parse('2026-06-01T12:00:00Z')

describe('parseSession', () => {
  it('reads a valid, unexpired session', () => {
    const raw = JSON.stringify({ token: 'sils_x', expiresAt: '2026-06-02T12:00:00Z', email: 'a@b.co' })
    expect(parseSession(raw, NOW)).toEqual({ token: 'sils_x', expiresAt: '2026-06-02T12:00:00Z', email: 'a@b.co' })
  })

  it('drops an expired session', () => {
    const raw = JSON.stringify({ token: 'sils_x', expiresAt: '2026-06-01T11:00:00Z', email: 'a@b.co' })
    expect(parseSession(raw, NOW)).toBeNull()
  })

  it('drops malformed or missing data', () => {
    expect(parseSession(null, NOW)).toBeNull()
    expect(parseSession('not json', NOW)).toBeNull()
    expect(parseSession('{"token":1}', NOW)).toBeNull()
    expect(parseSession('{"token":"t","expiresAt":"2026-06-02T00:00:00Z"}', NOW)).toBeNull()
  })
})
