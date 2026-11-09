import { describe, expect, it } from 'vitest'
import { DEFAULT_PREFERENCES, parsePreferences } from './preferences'

const WORKLOADS = ['calcul/sha256', 'rendu/raytracer']

describe('parsePreferences', () => {
  it('gives the defaults when nothing is stored', () => {
    expect(parsePreferences(null, WORKLOADS)).toEqual(DEFAULT_PREFERENCES)
  })

  it('keeps a known default workload', () => {
    expect(parsePreferences('{"defaultWorkload":"rendu/raytracer"}', WORKLOADS)).toEqual({
      defaultWorkload: 'rendu/raytracer',
    })
  })

  it('drops an unknown workload back to the first of the list', () => {
    expect(parsePreferences('{"defaultWorkload":"ia/llm"}', WORKLOADS)).toEqual(DEFAULT_PREFERENCES)
  })

  it('ignores a value that is not a string', () => {
    expect(parsePreferences('{"defaultWorkload":42}', WORKLOADS)).toEqual(DEFAULT_PREFERENCES)
  })

  it('ignores unreadable storage', () => {
    expect(parsePreferences('{not json', WORKLOADS)).toEqual(DEFAULT_PREFERENCES)
  })
})
