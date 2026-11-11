import { describe, expect, it } from 'vitest'
import { PASSAGES_FILE_NAME, passagesFile } from './embedding'

describe('passagesFile', () => {
  it('keeps one non-empty line per passage', async () => {
    const check = passagesFile('Premier texte\n\n  Deuxième texte  \r\n', 1024)
    expect(check.error).toBe('')
    expect(check.count).toBe(2)
    expect(check.file?.name).toBe(PASSAGES_FILE_NAME)
    expect(await check.file?.text()).toBe('Premier texte\nDeuxième texte')
  })

  it('refuses a text with no passage', () => {
    expect(passagesFile(' \n\n ', 1024)).toEqual({ file: null, count: 0, bytes: 0, error: 'empty' })
  })

  it('refuses a text over the batch size, counting UTF-8 bytes', () => {
    const check = passagesFile('é'.repeat(10), 15)
    expect(check.error).toBe('tooLarge')
    expect(check.file).toBeNull()
    expect(check.bytes).toBe(20)
  })
})