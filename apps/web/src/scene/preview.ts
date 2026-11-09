// Aperçu d'une scène : le moteur rend la scène en qualité brouillon, à une taille réduite (même
// cadrage, même rapport d'aspect). Le rendu final, lui, est celui des nœuds, à la taille demandée.

import { sceneToCfg } from './cfg'
import { renderRows } from './engine'
import type { SceneModel } from './types'

export interface PreviewImage {
  width: number
  height: number
  /** RGBA, prêt pour `ImageData`. */
  rgba: Uint8ClampedArray<ArrayBuffer>
}

/** Un aperçu de la scène, au plus `maxSide` pixels sur son plus grand côté. */
export async function renderPreview(model: SceneModel, maxSide = 320): Promise<PreviewImage> {
  const scale = Math.min(1, maxSide / Math.max(model.width, model.height))
  const width = Math.max(1, Math.round(model.width * scale))
  const height = Math.max(1, Math.round(model.height * scale))
  const text = sceneToCfg({ ...model, width, height, quality: 'draft' })
  const rgb = await renderRows(text, width, height, 0, height)
  const rgba = new Uint8ClampedArray(new ArrayBuffer(width * height * 4))
  for (let pixel = 0; pixel < width * height; pixel += 1) {
    rgba[pixel * 4] = rgb[pixel * 3]!
    rgba[pixel * 4 + 1] = rgb[pixel * 3 + 1]!
    rgba[pixel * 4 + 2] = rgb[pixel * 3 + 2]!
    rgba[pixel * 4 + 3] = 255
  }
  return { width, height, rgba }
}
