import { describe, expect, it } from 'vitest'
import {
  VECTOR_DIM,
  decodeVectors,
  passageRows,
  passagesOf,
  resultBytesOf,
  toCsv,
  toJson,
} from './embedding-result'

/** Un vecteur de 384 f32 little-endian, valeur `value` partout. */
function vectorBytes(value: number): Uint8Array {
  const bytes = new Uint8Array(VECTOR_DIM * 4)
  const view = new DataView(bytes.buffer)
  for (let dim = 0; dim < VECTOR_DIM; dim += 1) view.setFloat32(dim * 4, value, true)
  return bytes
}

const zeros = () => new Array<number>(VECTOR_DIM).fill(0)

describe('passagesOf', () => {
  it('keeps one trimmed non-empty line per passage, like the node', () => {
    expect(passagesOf('  un \r\n\r\ndeux\n   \ntrois')).toEqual(['un', 'deux', 'trois'])
  })
})

describe('decodeVectors', () => {
  it('reads each 384-value vector in order, little-endian', () => {
    const bytes = new Uint8Array([...vectorBytes(0.5), ...vectorBytes(-1.25)])
    const vectors = decodeVectors(bytes)
    expect(vectors).toHaveLength(2)
    expect(vectors[0]?.[0]).toBe(0.5)
    expect(vectors[1]?.[VECTOR_DIM - 1]).toBe(-1.25)
  })

  it('refuses a result that is not a whole number of vectors', () => {
    expect(() => decodeVectors(new Uint8Array(10))).toThrow()
  })
})

describe('passageRows', () => {
  it('pairs each passage with its vector', () => {
    const rows = passageRows(['un', 'deux'], [zeros(), zeros()])
    expect(rows.map((row) => [row.index, row.text])).toEqual([
      [0, 'un'],
      [1, 'deux'],
    ])
  })

  it('refuses a result that does not match the passages', () => {
    expect(() => passageRows(['un', 'deux'], [zeros()])).toThrow()
  })
})

describe('resultBytesOf', () => {
  it('joins the fragment lines in fragment order and skips the final digest line', () => {
    const text = [
      'Silicium, résultat du calcul ab',
      'workload: embedding/e5-small',
      'fragment 1: 0304',
      'fragment 0: 0102',
      'fragment 2: ',
      'résultat final (sha256 des fragments, dans l\'ordre): 00',
      '',
    ].join('\n')
    expect(Array.from(resultBytesOf(text))).toEqual([1, 2, 3, 4])
  })

  it('refuses text without any fragment line', () => {
    expect(() => resultBytesOf('rien ici')).toThrow()
  })
})

describe('exports', () => {
  it('writes a CSV with one row per passage and quotes the text', () => {
    const csv = toCsv([{ index: 0, text: 'il dit "oui"', vector: zeros() }])
    const [header, row] = csv.trim().split('\n')
    expect(header?.split(',')).toHaveLength(2 + VECTOR_DIM)
    expect(header?.startsWith('index,text,v0,')).toBe(true)
    expect(row?.startsWith('0,"il dit ""oui""",0,0,')).toBe(true)
  })

  it('neutralises a text that a spreadsheet would run as a formula', () => {
    const csv = toCsv([{ index: 3, text: '=HYPERLINK("x")', vector: zeros() }])
    expect(csv.split('\n')[1]?.startsWith('3,"\'=HYPERLINK(""x"")"')).toBe(true)
  })

  it('writes a JSON array of passages with their vectors', () => {
    const parsed = JSON.parse(toJson([{ index: 0, text: 'un', vector: zeros() }]))
    expect(parsed).toEqual([{ index: 0, text: 'un', vector: zeros() }])
  })
})
