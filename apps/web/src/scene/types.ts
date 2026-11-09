// Modèle de l'éditeur de scènes (port de la v0 : utils/scene.ts). Les coordonnées sont celles du
// moteur : la caméra regarde vers +x, +z est vers le haut, +y est à gauche de l'image.

export type Rgb = [number, number, number]
export type Xyz = [number, number, number]

export const MATERIAL_TYPES = ['phong', 'diffuse', 'metal', 'glass', 'emissive'] as const
export type MaterialType = (typeof MATERIAL_TYPES)[number]

export interface SceneMaterial {
  type: MaterialType
  /** 0–255 par canal. */
  color: Rgb
  /** Part de lumière réfléchie, 0–0.95 (phong). */
  reflection: number
  /** Force du reflet, 0–2 (phong). */
  specular: number
  /** Taille du reflet, 1 (large) – 512 (pointu) (phong). */
  shininess: number
  /** Flou des reflets, 0 (net) – 1 (phong, metal). */
  roughness: number
  /** Indice de réfraction, 1–3 (glass). */
  ior: number
  /** Lumière émise (emissive). */
  strength: number
}

interface Placed {
  id: number
  x: number
  y: number
  z: number
  material: SceneMaterial
}

export interface SceneSphere extends Placed {
  kind: 'sphere'
  radius: number
}

export interface ScenePlane extends Placed {
  kind: 'plane'
  normal: Xyz
}

/** Boîte centrée sur (x, y, z). */
export interface SceneBox extends Placed {
  kind: 'box'
  size: Xyz
  rotation: Xyz
}

/** Rectangle plat centré sur (x, y, z). */
export interface ScenePanel extends Placed {
  kind: 'panel'
  width: number
  height: number
  rotation: Xyz
}

/** Modèle triangulé : points mis à l'échelle `size`, tournés, puis déplacés en (x, y, z). */
export interface SceneMesh extends Placed {
  kind: 'mesh'
  vertices: number[]
  faces: number[]
  size: number
  rotation: Xyz
}

export type SceneObject = SceneSphere | ScenePlane | SceneBox | ScenePanel | SceneMesh
export type ObjectKind = SceneObject['kind']

export interface SceneLight {
  id: number
  /** `point` : un point de lumière (ombres nettes) ; `area` : une boule lumineuse (ombres douces). */
  kind: 'point' | 'area'
  x: number
  y: number
  z: number
  intensity: number
  color: Rgb
  /** Taille d'une lumière `area`. */
  radius: number
}

export interface SceneCamera {
  x: number
  y: number
  z: number
  /** Rotation vers la droite, en degrés. */
  yaw: number
  /** Regard vers le haut, en degrés. */
  pitch: number
  /** Ouverture : 0 = tout net ; plus = flou hors de `focus`. */
  aperture: number
  /** Distance du plan net (avec `aperture`). */
  focus: number
}

export interface SceneModel {
  width: number
  height: number
  /** Champ de vision vertical, 1–179 degrés. */
  fov: number
  camera: SceneCamera
  objects: SceneObject[]
  lights: SceneLight[]
  /** Qualité du rendu final (absent = `draft`). */
  quality?: RenderQuality
  /** Luminosité du ciel (0 = fond noir, pas de lumière du ciel). */
  sky?: number
  /** Luminosité générale de l'image. */
  exposure?: number
}

export type RenderQuality = 'draft' | 'smooth' | 'photo'

/** Réglages du moteur pour chaque qualité (bloc `render`). */
export const RENDER_QUALITIES: Record<
  RenderQuality,
  { samples: number; mode: 'whitted' | 'path'; bounces?: number }
> = {
  draft: { samples: 1, mode: 'whitted' },
  smooth: { samples: 16, mode: 'whitted' },
  photo: { samples: 256, mode: 'path', bounces: 6 },
}

/** Rayons tracés pour le rendu final (coût approximatif : le path tracing rebondit davantage). */
export function renderCost(scene: Pick<SceneModel, 'width' | 'height' | 'quality'>): number {
  const quality = RENDER_QUALITIES[scene.quality || 'draft']
  return scene.width * scene.height * quality.samples * (quality.mode === 'path' ? 4 : 1)
}

/** Matières prêtes à l'emploi : le type et les réglages qui donnent l'aspect. */
export const MATERIAL_PRESETS = {
  matte: { type: 'diffuse' },
  glossy: { type: 'phong', reflection: 0.25, specular: 0.8, shininess: 96, roughness: 0 },
  satin: { type: 'phong', reflection: 0.3, specular: 0.5, shininess: 40, roughness: 0.2 },
  mirror: { type: 'metal', roughness: 0 },
  metal: { type: 'metal', roughness: 0.3 },
  glass: { type: 'glass', ior: 1.5 },
  diamond: { type: 'glass', ior: 2.4 },
  light: { type: 'emissive', strength: 4 },
} as const satisfies Record<string, Partial<SceneMaterial>>
export type MaterialPreset = keyof typeof MATERIAL_PRESETS

/** Tailles d'image proposées. */
export const RESOLUTION_PRESETS = [
  { id: 'draft', width: 320, height: 200 },
  { id: 'hd', width: 1280, height: 720 },
  { id: 'fullHd', width: 1920, height: 1080 },
  { id: 'square', width: 1080, height: 1080 },
] as const

/** Orientations de plan proposées (vecteurs normaux). */
export const PLANE_ORIENTATIONS = {
  floor: [0, 0, 1],
  ceiling: [0, 0, -1],
  back: [-1, 0, 0],
  left: [0, -1, 0],
  right: [0, 1, 0],
} as const satisfies Record<string, Xyz>
export type PlaneOrientation = keyof typeof PLANE_ORIENTATIONS
