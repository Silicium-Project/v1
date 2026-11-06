import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { COLORS, PAIRS, contrastRatio } from './palette'

// Tests run from the package root (npm test): the stylesheet is read as plain text.
const tokens = readFileSync(resolve(process.cwd(), 'src/styles/tokens.css'), 'utf8')

describe('colour contrast (WCAG 2.2)', () => {
  it.each(PAIRS.map(([fg, bg, min]) => [fg, bg, min] as const))(
    '%s on %s reaches %s:1',
    (fg, bg, min) => {
      expect(contrastRatio(COLORS[fg], COLORS[bg])).toBeGreaterThanOrEqual(min)
    },
  )

  it('keeps tokens.css in step with palette.ts', () => {
    for (const value of Object.values(COLORS)) {
      expect(tokens.toLowerCase(), value).toContain(value)
    }
  })
})
