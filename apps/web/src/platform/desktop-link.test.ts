import { describe, expect, it } from 'vitest'
import { desktopLinkUrl } from './desktop-link'

describe('desktopLinkUrl', () => {
  it('opens the desktop application on the code', () => {
    expect(desktopLinkUrl('ABCDEFGHJKLM')).toBe('silicium://link?code=ABCDEFGHJKLM')
  })

  it('never lets the code add parameters', () => {
    expect(desktopLinkUrl('AB&CD')).toBe('silicium://link?code=AB%26CD')
  })
})
