// Écriture d'une scène en fichier `.cfg` (port de la v0 : sceneToCfg). Le texte produit est celui
// que le moteur lit ; la vérification et l'aperçu passent par le moteur lui-même.

import { RENDER_QUALITIES, type Rgb, type SceneLight, type SceneModel, type SceneMaterial, type SceneObject, type Xyz } from './types'

/** Un nombre tel que le fichier l'écrit : au plus 3 décimales, sans `-0`. */
export function formatNumber(value: number, decimals = 3): string {
  const rounded = Number((Number.isFinite(value) ? value : 0).toFixed(decimals))
  return String(Object.is(rounded, -0) ? 0 : rounded)
}

const clampNumber = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, Number.isFinite(value) ? value : min))
const point = (item: { x: number; y: number; z: number }) =>
  `x = ${formatNumber(item.x)} y = ${formatNumber(item.y)} z = ${formatNumber(item.z)}`
const vector = (name: string, value: Xyz) =>
  `${name} = { x = ${formatNumber(value[0])} y = ${formatNumber(value[1])} z = ${formatNumber(value[2])} }`
const turned = (rotation: Xyz) => rotation.some((angle) => formatNumber(angle) !== '0')
const channel = (value: number) => Math.round(clampNumber(value, 0, 255))
const color = (rgb: Rgb) =>
  `color = { r = ${channel(rgb[0])} g = ${channel(rgb[1])} b = ${channel(rgb[2])} }`

/** Les clés de matière d'un objet. */
function material(value: SceneMaterial): string {
  switch (value.type) {
    case 'diffuse':
      return `type = diffuse ${color(value.color)}`
    case 'metal':
      return `type = metal roughness = ${formatNumber(clampNumber(value.roughness, 0, 1))} ${color(value.color)}`
    case 'glass':
      return `type = glass ior = ${formatNumber(clampNumber(value.ior, 1, 3))} ${color(value.color)}`
    case 'emissive':
      return `type = emissive strength = ${formatNumber(Math.max(0, value.strength))} ${color(value.color)}`
    default: {
      const rough =
        value.roughness > 0 ? ` roughness = ${formatNumber(clampNumber(value.roughness, 0, 1))}` : ''
      return `type = phong reflection = ${formatNumber(clampNumber(value.reflection, 0, 0.95))} specular = ${formatNumber(clampNumber(value.specular, 0, 2))} shininess = ${formatNumber(clampNumber(value.shininess, 1, 512), 1)}${rough} ${color(value.color)}`
    }
  }
}

function primitive(item: SceneObject): string {
  const rotate = (rotation: Xyz) => (turned(rotation) ? `${vector('rotate', rotation)} ` : '')
  const translate = vector('translate', [item.x, item.y, item.z])
  switch (item.kind) {
    case 'sphere':
      return `\tsphere = { ${point(item)} radius = ${formatNumber(Math.max(0.001, item.radius))} ${material(item.material)} }`
    case 'plane':
      return `\tplane = { ${point(item)} normal_x = ${formatNumber(item.normal[0])} normal_y = ${formatNumber(item.normal[1])} normal_z = ${formatNumber(item.normal[2])} ${material(item.material)} }`
    case 'box':
      return `\tbox = { ${translate} ${vector('scale', item.size.map((side) => Math.max(0.001, Math.abs(side))) as Xyz)} ${rotate(item.rotation)}${material(item.material)} }`
    case 'panel':
      return `\tquad = { ${translate} scale = { x = 1 y = ${formatNumber(Math.max(0.001, item.width))} z = ${formatNumber(Math.max(0.001, item.height))} } ${rotate(item.rotation)}${material(item.material)} }`
    case 'mesh': {
      const lines = [
        `\tmesh = { ${translate} scale = ${formatNumber(Math.max(0.001, item.size))} ${rotate(item.rotation)}${material(item.material)}`,
        '\t\tvertices = {',
      ]
      for (let i = 0; i + 2 < item.vertices.length; i += 3)
        lines.push(
          `\t\t\t${formatNumber(item.vertices[i]!, 5)} ${formatNumber(item.vertices[i + 1]!, 5)} ${formatNumber(item.vertices[i + 2]!, 5)}`,
        )
      lines.push('\t\t}', '\t\tfaces = {')
      for (let i = 0; i + 2 < item.faces.length; i += 3)
        lines.push(`\t\t\t${item.faces[i]} ${item.faces[i + 1]} ${item.faces[i + 2]}`)
      lines.push('\t\t}', '\t}')
      return lines.join('\n')
    }
  }
}

function light(item: SceneLight): string {
  const tint = `r = ${channel(item.color[0])} g = ${channel(item.color[1])} b = ${channel(item.color[2])}`
  const intensity = `intensity = ${formatNumber(Math.max(0, item.intensity), 1)}`
  return item.kind === 'area'
    ? `\tarea = { ${point(item)} radius = ${formatNumber(Math.max(0.01, item.radius))} ${intensity} ${tint} }`
    : `\tpoint = { ${point(item)} ${intensity} ${tint} }`
}

/** Le fichier `.cfg` d'une scène (les commentaires restent hors des blocs). */
export function sceneToCfg(scene: SceneModel, title = ''): string {
  const width = Math.round(clampNumber(scene.width, 1, 16384))
  const height = Math.round(clampNumber(scene.height, 1, 16384))
  const name = title.replace(/[\r\n]+/g, ' ').trim()
  const quality = scene.quality || 'draft'
  const settings = RENDER_QUALITIES[quality]
  const sky = scene.sky ?? 1
  const exposure = scene.exposure ?? 1
  const camera = scene.camera
  const render = quality !== 'draft' || sky !== 1 || exposure !== 1
  const lines = [
    `# ${name || 'Silicium scene'}`,
    '# Written by the Silicium scene editor.',
    '',
    ...(render
      ? [
          'render :',
          '{',
          '\tengine = 2',
          `\tmode = ${settings.mode}`,
          `\tsamples = ${settings.samples}`,
          ...(settings.bounces ? [`\tbounces = ${settings.bounces}`] : []),
          ...(sky !== 1 ? [`\tsky = ${formatNumber(Math.max(0, sky))}`] : []),
          ...(exposure !== 1 ? [`\texposure = ${formatNumber(Math.max(0, exposure))}`] : []),
          '};',
          '',
        ]
      : []),
    'camera :',
    '{',
    `\tresolution = { width = ${width} height = ${height} }`,
    `\tposition = { ${point(camera)} }`,
    '\tdirection = { x = 1 y = 0 z = 0 }',
    // Rz·Ry autour de la caméra : z tourne à gauche, y regarde vers le bas.
    ...(formatNumber(camera.yaw) !== '0' || formatNumber(camera.pitch) !== '0'
      ? [`\trotation = { x = 0 y = ${formatNumber(-camera.pitch)} z = ${formatNumber(-camera.yaw)} }`]
      : []),
    `\tfieldOfView = ${formatNumber(clampNumber(scene.fov, 1, 179), 1)}`,
    ...(camera.aperture > 0
      ? [
          `\taperture = ${formatNumber(camera.aperture)}`,
          `\tfocus_distance = ${formatNumber(Math.max(0.01, camera.focus))}`,
        ]
      : []),
    '};',
    '',
    'primitives :',
    '{',
    ...scene.objects.map(primitive),
    '};',
    '',
    'lights :',
    '{',
    ...scene.lights.map(light),
    '};',
    '',
  ]
  return lines.join('\n')
}
