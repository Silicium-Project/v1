import { describe, expect, it } from 'vitest'
import { parseDraft } from './submit-draft'
import { sceneTemplate } from '@/scene/templates'

const WORKLOADS = ['calcul/sha256', 'rendu/raytracer']

function stored(overrides: Record<string, unknown> = {}): string {
  return JSON.stringify({ workload: 'rendu/raytracer', label: 'essai', scene: sceneTemplate('demo'), ...overrides })
}

describe('parseDraft', () => {
  it('reads a saved draft back', () => {
    const draft = parseDraft(stored(), WORKLOADS)
    expect(draft?.workload).toBe('rendu/raytracer')
    expect(draft?.label).toBe('essai')
    expect(draft?.scene.objects.length).toBeGreaterThan(0)
  })

  it('gives nothing when no draft is saved', () => {
    expect(parseDraft(null, WORKLOADS)).toBeNull()
  })

  it('refuses a workload the network does not know', () => {
    expect(parseDraft(stored({ workload: 'ia/llm' }), WORKLOADS)).toBeNull()
  })

  it('refuses a scene without its camera or object list', () => {
    expect(parseDraft(stored({ scene: { width: 64, height: 40 } }), WORKLOADS)).toBeNull()
  })

  it('refuses unreadable storage', () => {
    expect(parseDraft('{not json', WORKLOADS)).toBeNull()
  })
})
