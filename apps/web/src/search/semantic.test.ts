import { describe, expect, it } from 'vitest'
import { DIM, MAX_TOKENS, meanPoolNormalize, rank, score, truncateIds, type Passage } from './semantic'

/** Vecteur unitaire pointant vers la dimension `axis`. */
function axis(index: number): number[] {
  const vector = new Array<number>(DIM).fill(0)
  vector[index] = 1
  return vector
}

describe('meanPoolNormalize', () => {
  it('averages the unmasked tokens and normalises the result', () => {
    const hidden = new Float32Array(3 * DIM)
    hidden.fill(1, 0, DIM)
    hidden.fill(3, DIM, 2 * DIM)
    hidden.fill(1000, 2 * DIM)
    const pooled = meanPoolNormalize(hidden, [1, 1, 0])
    const expected = 1 / Math.sqrt(DIM)
    expect(pooled[0]).toBeCloseTo(expected, 6)
    expect(pooled[DIM - 1]).toBeCloseTo(expected, 6)
  })

  it('gives a zero vector when no token is real', () => {
    const pooled = meanPoolNormalize(new Float32Array(DIM).fill(5), [0])
    expect(pooled.every((value) => value === 0)).toBe(true)
  })
})

describe('truncateIds', () => {
  it('keeps short inputs as they are', () => {
    expect(truncateIds([0, 5, 6, 2], 0, 2)).toEqual([0, 5, 6, 2])
  })

  it('cuts the content and keeps the start and end tokens', () => {
    const ids = [0, ...Array.from({ length: 600 }, (_, i) => i + 10), 2]
    const cut = truncateIds(ids, 0, 2)
    expect(cut).toHaveLength(MAX_TOKENS)
    expect(cut[0]).toBe(0)
    expect(cut.at(-1)).toBe(2)
  })
})

describe('score and rank', () => {
  it('scores a vector against itself as 1 and orthogonal vectors as 0', () => {
    expect(score(Float32Array.from(axis(3)), axis(3))).toBeCloseTo(1, 6)
    expect(score(Float32Array.from(axis(3)), axis(4))).toBeCloseTo(0, 6)
  })

  const passages: Passage[] = [
    { index: 0, text: 'hôtel', vector: axis(0) },
    { index: 1, text: 'train', vector: axis(1) },
    { index: 2, text: 'plage', vector: axis(2) },
  ]

  it('returns the passages above the threshold, best first', () => {
    const query = Float32Array.from(axis(0))
    const hits = rank(query, passages, 0.5)
    expect(hits.map((hit) => hit.text)).toEqual(['hôtel'])
  })

  it('returns nothing when no passage reaches the threshold, so the answer is “not in the documents”', () => {
    const query = Float32Array.from(axis(9))
    expect(rank(query, passages)).toEqual([])
  })

  it('limits the number of hits', () => {
    const near = passages.map((passage, index) => ({
      ...passage,
      vector: axis(0).map((_, dim) => (dim === 0 ? 1 - index * 0.01 : 0)),
    }))
    const hits = rank(Float32Array.from(axis(0)), near, 0, 2)
    expect(hits).toHaveLength(2)
    expect(hits[0]?.index).toBe(0)
  })
})
