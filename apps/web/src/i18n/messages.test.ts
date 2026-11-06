import { describe, expect, it } from 'vitest'
import { KNOWN_ERROR_CODES } from '@/api/errors'
import { en } from './en'
import { fr } from './fr'

// Flattens a catalogue to "a.b.c" -> text. Arrays are indexed (sections.0.heading...).
function leaves(node: unknown, prefix = ''): Map<string, string> {
  const out = new Map<string, string>()
  if (typeof node === 'string') {
    out.set(prefix, node)
    return out
  }
  if (node !== null && typeof node === 'object') {
    for (const [key, value] of Object.entries(node)) {
      for (const [inner, text] of leaves(value, prefix ? `${prefix}.${key}` : key)) {
        out.set(inner, text)
      }
    }
  }
  return out
}

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort()

describe('message catalogues', () => {
  const frLeaves = leaves(fr)
  const enLeaves = leaves(en)

  it('have exactly the same keys in French and English', () => {
    expect([...enLeaves.keys()].sort()).toEqual([...frLeaves.keys()].sort())
  })

  it('have no empty text', () => {
    for (const [key, text] of [...frLeaves, ...enLeaves]) {
      expect(text.trim(), key).not.toBe('')
    }
  })

  it('use the same placeholders in both languages', () => {
    for (const [key, text] of frLeaves) {
      expect(placeholders(enLeaves.get(key) ?? ''), key).toEqual(placeholders(text))
    }
  })

  it('have a message for every error code the client can produce', () => {
    const codes = Object.keys(fr.errors).filter((key) => key !== 'generic').sort()
    expect(codes).toEqual([...KNOWN_ERROR_CODES].sort())
  })

  it('state the sign-up consent in both languages', () => {
    expect(fr.auth.consent).toContain('politique de confidentialité')
    expect(en.auth.consent).toContain('privacy policy')
  })
})
