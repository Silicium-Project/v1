import { describe, expect, it } from 'vitest'
import { ZipError, readZip } from './zip'

interface File {
  name: string
  data: Uint8Array
  deflate?: boolean
}

/** Écrit une archive ZIP minimale : entrées locales, répertoire central, fin de répertoire. */
async function zipOf(files: File[]): Promise<Uint8Array> {
  const parts: Uint8Array[] = []
  const central: Uint8Array[] = []
  let offset = 0
  for (const file of files) {
    const name = new TextEncoder().encode(file.name)
    const body = file.deflate ? await deflateRaw(file.data) : file.data
    const method = file.deflate ? 8 : 0
    const local = new DataView(new ArrayBuffer(30))
    local.setUint32(0, 0x04034b50, true)
    local.setUint16(8, method, true)
    local.setUint32(18, body.byteLength, true)
    local.setUint32(22, file.data.byteLength, true)
    local.setUint16(26, name.byteLength, true)
    parts.push(new Uint8Array(local.buffer), name, body)

    const record = new DataView(new ArrayBuffer(46))
    record.setUint32(0, 0x02014b50, true)
    record.setUint16(10, method, true)
    record.setUint32(20, body.byteLength, true)
    record.setUint32(24, file.data.byteLength, true)
    record.setUint16(28, name.byteLength, true)
    record.setUint32(42, offset, true)
    central.push(new Uint8Array(record.buffer), name)
    offset += 30 + name.byteLength + body.byteLength
  }
  const directorySize = central.reduce((sum, part) => sum + part.byteLength, 0)
  const end = new DataView(new ArrayBuffer(22))
  end.setUint32(0, 0x06054b50, true)
  end.setUint16(8, files.length, true)
  end.setUint16(10, files.length, true)
  end.setUint32(12, directorySize, true)
  end.setUint32(16, offset, true)
  return concat([...parts, ...central, new Uint8Array(end.buffer)])
}

async function deflateRaw(data: Uint8Array): Promise<Uint8Array> {
  const stream = new Blob([data.slice()]).stream().pipeThrough(new CompressionStream('deflate-raw'))
  return new Uint8Array(await new Response(stream).arrayBuffer())
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

const text = (value: string) => new TextEncoder().encode(value)
const names = (entries: { name: string }[]) => entries.map((entry) => entry.name)

describe('readZip', () => {
  it('reads stored and deflated entries, sorted by name', async () => {
    const archive = await zipOf([
      { name: '10.png', data: text('dix') },
      { name: '2.png', data: text('deux deux deux deux deux'), deflate: true },
    ])
    const entries = await readZip(archive, 1 << 20)
    expect(names(entries)).toEqual(['2.png', '10.png'])
    expect(new TextDecoder().decode(entries[0]?.data)).toBe('deux deux deux deux deux')
    expect(new TextDecoder().decode(entries[1]?.data)).toBe('dix')
  })

  it('skips folders, macOS metadata and paths that leave the archive', async () => {
    const archive = await zipOf([
      { name: 'dossier/', data: new Uint8Array() },
      { name: '__MACOSX/._a.png', data: text('x') },
      { name: '../evil.png', data: text('y') },
      { name: 'a.png', data: text('z') },
    ])
    expect(names(await readZip(archive, 1 << 20))).toEqual(['a.png'])
  })

  it('refuses an archive whose total size is over the limit', async () => {
    const archive = await zipOf([{ name: 'big.bin', data: text('0123456789') }])
    await expect(readZip(archive, 4)).rejects.toBeInstanceOf(ZipError)
  })

  it('refuses bytes that are not a zip archive', async () => {
    await expect(readZip(text('pas un zip'), 1 << 20)).rejects.toBeInstanceOf(ZipError)
  })
})
