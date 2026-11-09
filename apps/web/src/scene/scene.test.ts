import { describe, expect, it } from 'vitest'
import { formatNumber, sceneToCfg } from './cfg'
import { look, newSphere } from './factory'
import { builtinModel, BUILTIN_MODELS } from './models'
import { meshFromBlock } from './mesh'
import { SCENE_TEMPLATES, sceneTemplate } from './templates'
import { RENDER_QUALITIES, type SceneModel } from './types'

describe('scene writer', () => {
  it('writes the resolution, the camera and every object of a template', () => {
    const cfg = sceneToCfg(sceneTemplate('demo'), 'Démo')
    expect(cfg).toContain('resolution = { width = 320 height = 200 }')
    expect(cfg).toMatch(/fieldOfView = 55\n/)
    expect(cfg.match(/\tsphere = /g)).toHaveLength(3)
    expect(cfg.match(/\tplane = /g)).toHaveLength(1)
    expect(cfg.match(/\tpoint = /g)).toHaveLength(2)
    expect(cfg).not.toContain('render :')
  })

  it('adds a render block only when the quality or the sky differ from the defaults', () => {
    const scene = sceneTemplate('demo')
    scene.quality = 'photo'
    const cfg = sceneToCfg(scene)
    expect(cfg).toContain(`samples = ${RENDER_QUALITIES.photo.samples}`)
    expect(cfg).toContain('mode = path')
  })

  it('writes numbers with at most three decimals and never a negative zero', () => {
    expect(formatNumber(1.23456)).toBe('1.235')
    expect(formatNumber(-0)).toBe('0')
    expect(formatNumber(Number.NaN)).toBe('0')
  })

  it('writes a mesh with its vertices and faces', () => {
    const scene: SceneModel = sceneTemplate('blank')
    scene.objects.push(newSphere({ material: look([1, 2, 3], 'glass') }))
    const cfg = sceneToCfg(scene)
    expect(cfg).toContain('type = glass ior = 1.5 color = { r = 1 g = 2 b = 3 }')
  })
})

describe('templates and built-in models', () => {
  it('gives a fresh copy each time, so editing one never changes the template', () => {
    const first = sceneTemplate('showcase')
    first.objects[0]!.x = 99
    expect(sceneTemplate('showcase').objects[0]!.x).not.toBe(99)
  })

  it('knows every template and every built-in model', () => {
    for (const name of SCENE_TEMPLATES) expect(sceneTemplate(name).objects.length).toBeGreaterThanOrEqual(0)
    for (const name of BUILTIN_MODELS) {
      const model = builtinModel(name)
      expect(model.vertices.length % 3).toBe(0)
      expect(model.faces.length % 3).toBe(0)
    }
  })
})

describe('mesh blocks written by the engine', () => {
  it('reads the vertices and the faces of a mesh block', () => {
    const block = 'mesh = {\n\tmaterial = x\n\tvertices = {\n\t\t0 0 0\n\t\t1 0 0\n\t\t0 1 0\n\t}\n\tfaces = {\n\t\t0 1 2\n\t}\n}'
    expect(meshFromBlock(block)).toEqual({ vertices: [0, 0, 0, 1, 0, 0, 0, 1, 0], faces: [0, 1, 2] })
  })

  it('refuses a block without faces', () => {
    expect(() => meshFromBlock('mesh = { vertices = { 0 0 0 } }')).toThrow()
  })
})
