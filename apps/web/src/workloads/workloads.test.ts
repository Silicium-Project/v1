import { describe, expect, it } from 'vitest'
import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { WORKLOADS } from './generated'
import { findWorkload, isImageSide } from './find'
import { sceneResolution } from './scene'

// Tests run from the package root (npm test): the descriptors live two levels above apps/web.
const descriptorsDir = resolve(process.cwd(), '..', '..', 'contracts', 'workloads')
const descriptors = readdirSync(descriptorsDir)
  .filter((name) => name.endsWith('.json'))
  .map((name) => JSON.parse(readFileSync(resolve(descriptorsDir, name), 'utf8')))

describe('generated workload descriptors', () => {
  it('list every contract descriptor, and nothing else', () => {
    expect(WORKLOADS.map((spec) => spec.id)).toEqual(descriptors.map((raw) => raw.id).sort())
  })

  it('carry the same rules as the contract files they come from', () => {
    for (const raw of descriptors) {
      const spec = findWorkload(raw.id)
      expect(spec, raw.id).toBeDefined()
      expect(spec?.unit, raw.id).toBe(raw.unit)
      expect(spec?.maxFragments, raw.id).toBe(raw.split.max_fragments)
      expect(spec?.targetSize, raw.id).toBe(raw.split.target_size)
      expect(spec?.rewardPerUnit, raw.id).toBe(raw.terms.reward_per_unit)
      expect(spec?.verifierRewardPerUnit, raw.id).toBe(raw.terms.verifier_reward_per_unit)
      expect(spec?.depositSupported, raw.id).toBe(raw.deposit === 'supported')
    }
  })

  it('refuses an identifier nobody declared', () => {
    expect(findWorkload('ia/llm')).toBeUndefined()
  })

  it('accepts image sides only up to the descriptor maximum', () => {
    const image = findWorkload('rendu/raytracer')
    expect(image).toBeDefined()
    if (!image) return
    expect(isImageSide(1, image)).toBe(true)
    expect(isImageSide(image.maxSide, image)).toBe(true)
    expect(isImageSide(image.maxSide + 1, image)).toBe(false)
    expect(isImageSide(0, image)).toBe(false)
    expect(isImageSide(1.5, image)).toBe(false)
  })
})

describe('scene resolution', () => {
  it('reads the resolution the scene declares, as the engine does', () => {
    const scene = 'camera :\n{\n\tresolution = { width = 320 height = 200 }\n\tfieldOfView = 55.0\n};\n'
    expect(sceneResolution(scene)).toEqual({ width: 320, height: 200 })
  })

  it('is undefined when the scene declares none', () => {
    expect(sceneResolution('primitives : { sphere = { radius = 1 } }')).toBeUndefined()
  })
})
