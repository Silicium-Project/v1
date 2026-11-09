/**
 * The resolution a raytracing scene declares (`camera : { resolution = { width = W height = H } }`),
 * read the way the engine reads it (sil-raytracer, scene build). Undefined when the text declares
 * none: the user then types the size by hand.
 */
export function sceneResolution(text: string): { width: number; height: number } | undefined {
  const match = /resolution\s*=\s*\{[^}]*width\s*=\s*(\d+)[^}]*height\s*=\s*(\d+)/i.exec(text)
  if (!match) return undefined
  return { width: Number(match[1]), height: Number(match[2]) }
}
