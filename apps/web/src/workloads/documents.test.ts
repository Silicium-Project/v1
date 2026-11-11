import { describe, expect, it } from 'vitest'
import { DocumentError, MAX_UPLOAD_BYTES, passagesOf, textLines } from './documents'

const enc = (value: string) => new TextEncoder().encode(value)

/** Archive ZIP minimale, entrées stockées (sans compression). */
function zipOf(files: Record<string, string>): Uint8Array {
  const parts: Uint8Array[] = []
  const central: Uint8Array[] = []
  let offset = 0
  for (const [name, content] of Object.entries(files)) {
    const nameBytes = enc(name)
    const body = enc(content)
    const local = new DataView(new ArrayBuffer(30))
    local.setUint32(0, 0x04034b50, true)
    local.setUint32(18, body.byteLength, true)
    local.setUint32(22, body.byteLength, true)
    local.setUint16(26, nameBytes.byteLength, true)
    parts.push(new Uint8Array(local.buffer), nameBytes, body)
    const record = new DataView(new ArrayBuffer(46))
    record.setUint32(0, 0x02014b50, true)
    record.setUint32(20, body.byteLength, true)
    record.setUint32(24, body.byteLength, true)
    record.setUint16(28, nameBytes.byteLength, true)
    record.setUint32(42, offset, true)
    central.push(new Uint8Array(record.buffer), nameBytes)
    offset += 30 + nameBytes.byteLength + body.byteLength
  }
  const directorySize = central.reduce((sum, part) => sum + part.byteLength, 0)
  const end = new DataView(new ArrayBuffer(22))
  end.setUint32(0, 0x06054b50, true)
  end.setUint16(8, Object.keys(files).length, true)
  end.setUint16(10, Object.keys(files).length, true)
  end.setUint32(12, directorySize, true)
  end.setUint32(16, offset, true)
  const all = [...parts, ...central, new Uint8Array(end.buffer)]
  const out = new Uint8Array(all.reduce((sum, part) => sum + part.byteLength, 0))
  let at = 0
  for (const part of all) {
    out.set(part, at)
    at += part.byteLength
  }
  return out
}

const file = (name: string, bytes: Uint8Array) => new File([bytes.slice()], name)

async function codeOf(promise: Promise<unknown>): Promise<string> {
  try {
    await promise
  } catch (error) {
    if (error instanceof DocumentError) return error.code
    throw error
  }
  return 'ok'
}

describe('textLines', () => {
  it('keeps one trimmed line per non-empty line, with CRLF endings', () => {
    expect(textLines(enc('  un \r\n\r\nDeux\n   \ntrois'))).toEqual(['un', 'Deux', 'trois'])
  })

  it('refuses bytes that are not UTF-8', () => {
    expect(() => textLines(new Uint8Array([0xc3, 0x28]))).toThrow(DocumentError)
  })
})

describe('passagesOf', () => {
  it('reads a text file, one passage per line', async () => {
    expect(await passagesOf(file('notes.md', enc('Premier\n\nSecond\n')))).toBe('Premier\nSecond')
  })

  it('reads the text entries of a ZIP and ignores the others, in the order of the archive', async () => {
    const archive = zipOf({ 'b.txt': 'deux', 'a.md': 'un', 'image.png': 'pixels', 'nested.zip': 'x' })
    expect(await passagesOf(file('lot.zip', archive))).toBe('un\ndeux')
  })

  it('refuses a ZIP with no readable entry', async () => {
    const archive = zipOf({ 'image.png': 'pixels' })
    expect(await codeOf(passagesOf(file('lot.zip', archive)))).toBe('unsupported')
  })

  it('refuses a format it does not read', async () => {
    expect(await codeOf(passagesOf(file('rapport.docx', enc('x'))))).toBe('unsupported')
  })

  it('refuses a text file with no passage', async () => {
    expect(await codeOf(passagesOf(file('vide.txt', enc(' \n\n'))))).toBe('noText')
  })

  it('refuses a file over the upload limit before reading it', async () => {
    const big = file('gros.txt', enc('x'))
    Object.defineProperty(big, 'size', { value: MAX_UPLOAD_BYTES + 1 })
    expect(await codeOf(passagesOf(big))).toBe('tooLarge')
  })

  it('refuses a ZIP that is not an archive', async () => {
    expect(await codeOf(passagesOf(file('faux.zip', enc('pas un zip'))))).toBe('invalid')
  })
})
