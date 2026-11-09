// Single source of the colour values used in tokens.css. The contrast test reads this file
// and fails if a text/background pair drops below its WCAG 2.2 minimum.

export const COLORS = {
  bg: '#f4efe6',
  surface: '#fbf8f2',
  text: '#1d1a16',
  muted: '#5a5246',
  accent: '#a23b16',
  accentStrong: '#7f2c0f',
  accentText: '#ffffff',
  accentSoft: '#f3e2d5',
  inputBorder: '#7d7263',
  danger: '#a4262c',
  dangerSoft: '#fbe9e8',
  success: '#2f6b3a',
  successSoft: '#e6f1e4',
  warning: '#7a5200',
  warningSoft: '#fbefd6',
  neutral: '#5a5246',
  neutralSoft: '#ebe4d6',
} as const

export type ColorName = keyof typeof COLORS

/** [foreground, background, minimum ratio]. 4.5 for text (1.4.3), 3 for UI boundaries (1.4.11). */
export const PAIRS: ReadonlyArray<readonly [ColorName, ColorName, number]> = [
  ['text', 'bg', 4.5],
  ['text', 'surface', 4.5],
  ['text', 'accentSoft', 4.5],
  ['muted', 'bg', 4.5],
  ['muted', 'surface', 4.5],
  ['accentText', 'accent', 4.5],
  ['accentText', 'danger', 4.5],
  ['accent', 'surface', 4.5],
  ['accent', 'accentSoft', 4.5],
  ['danger', 'surface', 4.5],
  ['danger', 'dangerSoft', 4.5],
  ['success', 'surface', 4.5],
  ['success', 'successSoft', 4.5],
  ['warning', 'surface', 4.5],
  ['warning', 'warningSoft', 4.5],
  ['neutral', 'neutralSoft', 4.5],
  ['inputBorder', 'surface', 3],
]

function channel(value: number): number {
  const c = value / 255
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}

/** WCAG relative luminance of a `#rrggbb` colour. */
export function luminance(hex: string): number {
  const n = parseInt(hex.slice(1), 16)
  const r = channel((n >> 16) & 255)
  const g = channel((n >> 8) & 255)
  const b = channel(n & 255)
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

/** WCAG contrast ratio between two `#rrggbb` colours, between 1 and 21. */
export function contrastRatio(a: string, b: string): number {
  const la = luminance(a)
  const lb = luminance(b)
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05)
}
