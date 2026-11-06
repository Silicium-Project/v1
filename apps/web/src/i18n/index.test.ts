import { beforeEach, describe, expect, it } from 'vitest'
import { detectLocale, setLocale, t } from './index'

describe('detectLocale', () => {
  it('keeps a stored explicit choice', () => {
    expect(detectLocale('en', ['fr-FR'])).toBe('en')
  })

  it('ignores a stored value it does not know', () => {
    expect(detectLocale('de', ['en-US'])).toBe('en')
  })

  it('follows the browser language', () => {
    expect(detectLocale(null, ['en-GB'])).toBe('en')
    expect(detectLocale(null, ['fr-CA', 'en'])).toBe('fr')
  })

  it('falls back to French', () => {
    expect(detectLocale(null, ['de-DE'])).toBe('fr')
    expect(detectLocale(null, [])).toBe('fr')
  })
})

describe('t', () => {
  beforeEach(() => setLocale('fr'))

  it('fills placeholders', () => {
    expect(t('job.refreshed', { time: '10:42' })).toBe('Mis à jour à 10:42')
  })

  it('switches language', () => {
    setLocale('en')
    expect(t('job.refreshed', { time: '10:42' })).toBe('Updated at 10:42')
    expect(document.documentElement.lang).toBe('en')
  })

  it('returns the key for an unknown message instead of an empty string', () => {
    expect(t('nope.missing')).toBe('nope.missing')
  })
})
