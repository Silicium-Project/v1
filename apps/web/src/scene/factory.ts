// Fabriques des objets, lumières et caméras de l'éditeur, et matières prêtes à l'emploi.

import {
  MATERIAL_PRESETS,
  PLANE_ORIENTATIONS,
  type MaterialPreset,
  type PlaneOrientation,
  type Rgb,
  type SceneBox,
  type SceneCamera,
  type SceneLight,
  type SceneMaterial,
  type SceneMesh,
  type ScenePanel,
  type ScenePlane,
  type SceneModel,
  type SceneSphere,
  type Xyz,
} from './types'

const BASE_MATERIAL: Omit<SceneMaterial, 'color'> = {
  type: 'phong',
  reflection: 0.25,
  specular: 0.8,
  shininess: 96,
  roughness: 0,
  ior: 1.5,
  strength: 4,
}

/** Une matière : un préréglage (ou des réglages phong) dans une couleur. */
export function look(
  color: Rgb,
  preset: MaterialPreset | Partial<Omit<SceneMaterial, 'color'>> = 'glossy',
  patch: Partial<SceneMaterial> = {},
): SceneMaterial {
  const settings = typeof preset === 'string' ? MATERIAL_PRESETS[preset] : preset
  return { ...BASE_MATERIAL, ...settings, ...patch, color: [...(patch.color || color)] as Rgb }
}

/** Le préréglage auquel correspond une matière, ou `custom`. */
export function materialPreset(material: SceneMaterial): MaterialPreset | 'custom' {
  const found = (Object.keys(MATERIAL_PRESETS) as MaterialPreset[]).find((name) =>
    Object.entries(MATERIAL_PRESETS[name]).every(
      ([key, value]) => material[key as keyof SceneMaterial] === value,
    ),
  )
  return found || 'custom'
}

/** L'orientation d'un plan, ou `custom`. */
export function planeOrientation(plane: ScenePlane): PlaneOrientation | 'custom' {
  const found = (Object.keys(PLANE_ORIENTATIONS) as PlaneOrientation[]).find((name) =>
    PLANE_ORIENTATIONS[name].every((value, index) => value === plane.normal[index]),
  )
  return found || 'custom'
}

let nextId = 1
/** Un identifiant neuf pour un objet ou une lumière. */
export const newId = (): number => nextId++

type New<T> = Partial<Omit<T, 'id' | 'kind'>>

export const newSphere = (patch: New<SceneSphere> = {}): SceneSphere => ({
  id: newId(),
  kind: 'sphere',
  x: 5,
  y: 0,
  z: 0,
  radius: 0.8,
  material: look([70, 160, 255]),
  ...patch,
})

export const newPlane = (patch: New<ScenePlane> = {}): ScenePlane => ({
  id: newId(),
  kind: 'plane',
  x: 0,
  y: 0,
  z: -1,
  normal: [0, 0, 1],
  material: look([225, 232, 240], { reflection: 0.15, specular: 0.4, shininess: 48 }),
  ...patch,
})

export const newBox = (patch: New<SceneBox> = {}): SceneBox => ({
  id: newId(),
  kind: 'box',
  x: 5,
  y: 0,
  z: -0.5,
  size: [1, 1, 1],
  rotation: [0, 0, 20],
  material: look([255, 200, 70], 'satin'),
  ...patch,
})

export const newPanel = (patch: New<ScenePanel> = {}): ScenePanel => ({
  id: newId(),
  kind: 'panel',
  x: 8,
  y: 0,
  z: 1,
  width: 3,
  height: 2,
  rotation: [0, 0, 0],
  material: look([255, 244, 230], 'light'),
  ...patch,
})

export const newMesh = (patch: New<SceneMesh> = {}): SceneMesh => ({
  id: newId(),
  kind: 'mesh',
  x: 5,
  y: 0,
  z: -1,
  vertices: [],
  faces: [],
  size: 1.5,
  rotation: [0, 0, 0],
  material: look([200, 120, 255], 'glossy'),
  ...patch,
})

export const newLight = (patch: Partial<Omit<SceneLight, 'id'>> = {}): SceneLight => ({
  id: newId(),
  kind: 'point',
  x: 3,
  y: 0,
  z: 5,
  intensity: 400,
  color: [255, 255, 255],
  radius: 0.5,
  ...patch,
})

export const newCamera = (patch: Partial<SceneCamera> = {}): SceneCamera => ({
  x: 0,
  y: 0,
  z: 0,
  yaw: 0,
  pitch: 0,
  aperture: 0,
  focus: 5,
  ...patch,
})

/** Copie profonde : l'éditeur ne partage jamais de tableaux entre deux scènes. */
export const cloneScene = (scene: SceneModel): SceneModel =>
  JSON.parse(JSON.stringify(scene)) as SceneModel

export type { Xyz }
