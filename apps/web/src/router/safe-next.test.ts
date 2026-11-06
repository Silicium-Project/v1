import { describe, expect, it } from 'vitest'
import { safeNext } from './safe-next'

describe('safeNext', () => {
  it('keeps an in-app path', () => {
    expect(safeNext('/machines?tab=1')).toBe('/machines?tab=1')
  })

  it.each([
    ['https://evil.example'],
    ['//evil.example'],
    ['/\\evil.example'],
    ['javascript:alert(1)'],
    ['/calculs\nSet-Cookie'],
    ['calculs'],
    [undefined],
    [['/machines']],
  ])('refuses %j and falls back to the jobs page', (value) => {
    expect(safeNext(value)).toBe('/calculs')
  })
})
