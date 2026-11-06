import { describe, expect, it } from 'vitest'
import { decodeBase58, encodeBase58 } from './base58'

const bytes = (text: string) => new TextEncoder().encode(text)

describe('base58', () => {
  it('matches the reference vector used by the Go wallet package', () => {
    expect(encodeBase58(bytes('Hello World'))).toBe('JxF12TrwUP45BMd')
    expect(new TextDecoder().decode(decodeBase58('JxF12TrwUP45BMd'))).toBe('Hello World')
  })

  it('keeps leading zero bytes as leading 1 characters', () => {
    expect(encodeBase58(Uint8Array.of(0, 0, 1))).toBe('112')
    expect(Array.from(decodeBase58('112'))).toEqual([0, 0, 1])
  })

  it('round-trips a 32-byte key', () => {
    const key = new Uint8Array(32).map((_, i) => (i * 37 + 11) & 0xff)
    key[0] = 0
    expect(decodeBase58(encodeBase58(key))).toEqual(key)
  })

  it('refuses characters outside the alphabet', () => {
    for (const bad of ['', '0OIl', 'abc!']) {
      expect(() => decodeBase58(bad)).toThrow()
    }
  })
})
