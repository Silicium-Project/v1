import { describe, expect, it } from 'vitest'
import { DocumentError, MAX_UPLOAD_BYTES, passagesOf, textLines } from './documents'

const enc = (value: string) => new TextEncoder().encode(value)

/** Archive ZIP minimale, entrées stockées (sans compression) ; contenu texte ou binaire. */
function zipOf(files: Record<string, string | Uint8Array>): Uint8Array {
  const parts: Uint8Array[] = []
  const central: Uint8Array[] = []
  let offset = 0
  for (const [name, content] of Object.entries(files)) {
    const nameBytes = enc(name)
    const body = typeof content === 'string' ? enc(content) : content
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
  const count = Object.keys(files).length
  const directorySize = central.reduce((sum, part) => sum + part.byteLength, 0)
  const end = new DataView(new ArrayBuffer(22))
  end.setUint32(0, 0x06054b50, true)
  end.setUint16(8, count, true)
  end.setUint16(10, count, true)
  end.setUint32(12, directorySize, true)
  end.setUint32(16, offset, true)
  return concat([...parts, ...central, new Uint8Array(end.buffer)])
}

function concat(parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.byteLength, 0))
  let at = 0
  for (const part of parts) {
    out.set(part, at)
    at += part.byteLength
  }
  return out
}

const file = (name: string, bytes: Uint8Array) => new File([bytes.slice()], name)

/** Code et fichiers concernés d'une erreur de document. */
async function failureOf(promise: Promise<unknown>): Promise<{ code: string; names: string[] }> {
  try {
    await promise
  } catch (error) {
    if (error instanceof DocumentError) return { code: error.code, names: error.names }
    throw error
  }
  return { code: 'ok', names: [] }
}

describe('textLines', () => {
  it('keeps one trimmed line per non-empty line, with CRLF endings', () => {
    expect(textLines('  un \r\n\r\nDeux\n   \ntrois')).toEqual(['un', 'Deux', 'trois'])
  })
})

describe('passagesOf', () => {
  it('reads a text file of any text format, one passage per line', async () => {
    expect(await passagesOf([file('notes.csv', enc('a,b\n\nc,d\n'))])).toBe('a,b\nc,d')
  })

  it('reads several files, in the order they were chosen', async () => {
    const text = await passagesOf([file('deux.md', enc('deux')), file('un.txt', enc('un'))])
    expect(text).toBe('deux\nun')
  })

  it('reads every entry of a ZIP, of any type it can read, and the archives inside it', async () => {
    const inner = zipOf({ 'cache.txt': 'interieur' })
    const archive = zipOf({ 'b.txt': 'deux', 'a.md': 'un', 'dossier/c.json': '{"c":1}', 'imbriquee.zip': inner })
    expect(await passagesOf([file('lot.zip', archive)])).toBe('un\ndeux\n{"c":1}\ninterieur')
  })

  it('names the files it cannot read, binary ones included, and reads none of them silently', async () => {
    const archive = zipOf({ 'texte.txt': 'ok', 'photo.png': new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0xff]) })
    const failure = await failureOf(passagesOf([file('lot.zip', archive)]))
    expect(failure).toEqual({ code: 'unsupported', names: ['photo.png'] })
  })

  it('names an unreadable binary file passed directly', async () => {
    const failure = await failureOf(passagesOf([file('donnees.bin', new Uint8Array([0xff, 0xfe, 0x00, 0x81]))]))
    expect(failure).toEqual({ code: 'unsupported', names: ['donnees.bin'] })
  })

  it('reads the paragraphs of a Word document', async () => {
    const xml = '<w:document xmlns:w="http://x"><w:body><w:p><w:r><w:t>Premier</w:t></w:r></w:p><w:p><w:r><w:t>Second</w:t></w:r></w:p></w:body></w:document>'
    const docx = zipOf({ '[Content_Types].xml': '<x/>', 'word/document.xml': xml })
    expect(await passagesOf([file('rapport.docx', docx)])).toBe('Premier\nSecond')
  })

  it('reads the shared strings and the cells of an Excel sheet', async () => {
    const shared = '<sst xmlns="http://s"><si><t>Nom</t></si><si><t>Prix</t></si></sst>'
    const sheet =
      '<worksheet xmlns="http://s"><sheetData>' +
      '<row><c t="s"><v>0</v></c><c t="s"><v>1</v></c></row>' +
      '<row><c><v>42</v></c><c><v>3.5</v></c></row>' +
      '</sheetData></worksheet>'
    const xlsx = zipOf({ 'xl/sharedStrings.xml': shared, 'xl/worksheets/sheet1.xml': sheet })
    expect(await passagesOf([file('tarifs.xlsx', xlsx)])).toBe('Nom\tPrix\n42\t3.5')
  })

  it('reads the text of an HTML page without its scripts', async () => {
    const page = '<html><body><p>Un</p><p>Deux</p><script>x = 1</script></body></html>'
    expect(await passagesOf([file('page.html', enc(page))])).toBe('Un\nDeux')
  })

  it('refuses a file with no text at all, and names it', async () => {
    expect(await failureOf(passagesOf([file('vide.txt', enc(' \n\n'))]))).toEqual({ code: 'noText', names: [] })
  })

  it('refuses a file over the upload limit, and names it, before reading it', async () => {
    const big = file('gros.txt', enc('x'))
    Object.defineProperty(big, 'size', { value: MAX_UPLOAD_BYTES + 1 })
    expect(await failureOf(passagesOf([big]))).toEqual({ code: 'tooLarge', names: ['gros.txt'] })
  })

  it('refuses a ZIP that is not an archive, and names it', async () => {
    expect(await failureOf(passagesOf([file('faux.zip', enc('pas un zip'))]))).toEqual({
      code: 'invalid',
      names: ['faux.zip'],
    })
  })

  it('refuses an empty selection', async () => {
    expect(await failureOf(passagesOf([]))).toEqual({ code: 'noText', names: [] })
  })
})
