import { describe, expect, it } from 'vitest'
import { linesOfPages } from './pdf'

describe('linesOfPages', () => {
  it('joins the items of a line until hasEOL, then starts a new line', () => {
    const page = [
      { str: 'Le texte ', hasEOL: false },
      { str: 'continue ici', hasEOL: true },
      { str: 'Ligne suivante', hasEOL: true },
    ]
    expect(linesOfPages([page])).toEqual(['Le texte continue ici', 'Ligne suivante'])
  })

  it('starts a new line at each page and drops blank lines', () => {
    const first = [{ str: 'Page un', hasEOL: false }]
    const second = [{ str: '   ', hasEOL: true }, { str: 'Page deux', hasEOL: false }]
    expect(linesOfPages([first, second])).toEqual(['Page un', 'Page deux'])
  })

  it('gives no line for a page without text (a scan)', () => {
    expect(linesOfPages([[], [{}]])).toEqual([])
  })
})
