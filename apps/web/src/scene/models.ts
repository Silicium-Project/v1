// Modèles intégrés pour les objets `mesh` (port de la v0 : utils/engine/models.ts, partie
// formes). Les modèles sont en z-up, centrés sur l'origine, posés sur z = 0. L'import d'un fichier
// .obj passe par le moteur (`sil-raytracer-block obj`), pas par ce fichier.

export interface Model {
  vertices: number[]
  faces: number[]
}

export const BUILTIN_MODELS = ['pyramid', 'gem', 'cylinder', 'torus'] as const
export type BuiltinModel = (typeof BUILTIN_MODELS)[number]

/** Centre le modèle, le pose sur z = 0 et l'échelonne : sa plus grande dimension vaut `size`. */
function fit(model: Model, size = 1): Model {
  const low = [Infinity, Infinity, Infinity]
  const high = [-Infinity, -Infinity, -Infinity]
  for (let i = 0; i < model.vertices.length; i += 1) {
    low[i % 3] = Math.min(low[i % 3]!, model.vertices[i]!)
    high[i % 3] = Math.max(high[i % 3]!, model.vertices[i]!)
  }
  const largest = Math.max(0, ...[0, 1, 2].map((axis) => high[axis]! - low[axis]!))
  if (largest <= 0) return model
  const factor = size / largest
  const centre = [(low[0]! + high[0]!) / 2, (low[1]! + high[1]!) / 2, low[2]!]
  return {
    vertices: model.vertices.map((value, i) => Number(((value - centre[i % 3]!) * factor).toFixed(5))),
    faces: model.faces,
  }
}

/** Anneaux de points autour de l'axe z ; `closed` relie le dernier anneau au premier (tore). */
function revolve(profile: [number, number][], segments: number, closed: boolean): Model {
  const vertices: number[] = []
  const faces: number[] = []
  const rings = profile.length
  for (let s = 0; s < segments; s += 1) {
    const angle = (2 * Math.PI * s) / segments
    for (const [radius, z] of profile) vertices.push(radius * Math.cos(angle), radius * Math.sin(angle), z)
  }
  const at = (s: number, r: number) => (s % segments) * rings + (r % rings)
  for (let s = 0; s < segments; s += 1) {
    for (let r = 0; r < (closed ? rings : rings - 1); r += 1) {
      faces.push(at(s, r), at(s + 1, r), at(s + 1, r + 1), at(s, r), at(s + 1, r + 1), at(s, r + 1))
    }
  }
  return { vertices, faces }
}

/** Un modèle intégré, d'une unité de grandeur. */
export function builtinModel(name: BuiltinModel): Model {
  switch (name) {
    case 'pyramid':
      return fit({
        vertices: [-1, -1, 0, 1, -1, 0, 1, 1, 0, -1, 1, 0, 0, 0, 1.4],
        faces: [0, 2, 1, 0, 3, 2, 0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4],
      })
    case 'gem': {
      // Icosaèdre.
      const p = (1 + Math.sqrt(5)) / 2
      const vertices = [
        -1, p, 0, 1, p, 0, -1, -p, 0, 1, -p, 0,
        0, -1, p, 0, 1, p, 0, -1, -p, 0, 1, -p,
        p, 0, -1, p, 0, 1, -p, 0, -1, -p, 0, 1,
      ]
      const faces = [
        0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11, 1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7, 6, 7,
        1, 8, 3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9, 4, 9, 5, 2, 4, 11, 6, 2, 10, 8, 6, 7, 9,
        8, 1,
      ]
      return fit({ vertices, faces })
    }
    case 'cylinder':
      return fit(
        revolve(
          [
            [0, 0],
            [1, 0],
            [1, 2],
            [0, 2],
          ],
          32,
          false,
        ),
      )
    case 'torus': {
      const tube: [number, number][] = []
      for (let i = 0; i < 12; i += 1) {
        const angle = (2 * Math.PI * i) / 12
        tube.push([1 + 0.38 * Math.cos(angle), 0.38 * Math.sin(angle)])
      }
      return fit(revolve(tube, 32, true))
    }
  }
}
