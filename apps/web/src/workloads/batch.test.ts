import { describe, expect, it } from 'vitest'
import { FLAG_GRAYSCALE, NO_OPTIONS, batchKindOf, buildBatch, checkOptions } from './batch'

const record = (text: string) => new TextEncoder().encode(text)

describe('buildBatch', () => {
  it('writes the version 2 layout that sil-batch reads', () => {
    const bytes = buildBatch(
      {
        kind: 'image',
        format: 1,
        width: 4,
        height: 3,
        options: { ...NO_OPTIONS, quality: 80, flags: FLAG_GRAYSCALE },
      },
      [record('abc')],
    )
    const view = new DataView(bytes.buffer)
    expect(new TextDecoder().decode(bytes.subarray(0, 8))).toBe('SILBATCH')
    expect(view.getUint8(8)).toBe(2)
    expect(view.getUint8(9)).toBe(1)
    expect(view.getUint8(10)).toBe(1)
    expect(view.getUint32(12, true)).toBe(4)
    expect(view.getUint32(16, true)).toBe(3)
    expect(view.getUint32(20, true)).toBe(1)
    expect(view.getUint8(24)).toBe(80)
    expect(view.getUint8(27)).toBe(FLAG_GRAYSCALE)
    expect(view.getUint32(40, true)).toBe(3)
    expect(new TextDecoder().decode(bytes.subarray(44))).toBe('abc')
    expect(bytes.byteLength).toBe(47)
  })

  it('refuses an option that does not belong to an image batch', () => {
    expect(() =>
      buildBatch({ kind: 'image', format: 0, width: 0, height: 0, options: { ...NO_OPTIONS, channels: 2 } }, []),
    ).toThrow()
  })
})

describe('checkOptions', () => {
  it('accepts the defaults', () => {
    expect(checkOptions('image', 0, NO_OPTIONS)).toBeUndefined()
  })

  it('refuses out-of-range values', () => {
    expect(checkOptions('image', 1, { ...NO_OPTIONS, quality: 101 })).toBeDefined()
    expect(checkOptions('image', 9, NO_OPTIONS)).toBeDefined()
  })
})

describe('batchKindOf', () => {
  it('maps the image batch workload and nothing else', () => {
    expect(batchKindOf('image/batch')).toBe('image')
    expect(batchKindOf('calcul/sha256')).toBeUndefined()
    expect(batchKindOf('audio/transcode')).toBeUndefined()
  })
})
