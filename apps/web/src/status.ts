import type { JobStateName } from '@/api/endpoints'

export type Tone = 'info' | 'success' | 'neutral' | 'warning'

const TONES: Record<JobStateName, Tone> = {
  running: 'info',
  finalized: 'success',
  cancelled: 'neutral',
  expired: 'neutral',
  not_on_node: 'warning',
  unknown: 'warning',
}

export const JOB_STATES = Object.keys(TONES) as JobStateName[]

/** Any state the API does not list is shown as unknown: never guessed. */
export function normalizeState(state: string): JobStateName {
  return (JOB_STATES as string[]).includes(state) ? (state as JobStateName) : 'unknown'
}

export function stateTone(state: string): Tone {
  return TONES[normalizeState(state)]
}

export function stateLabelKey(state: string): string {
  return `status.${normalizeState(state)}.label`
}

export function stateDescriptionKey(state: string): string {
  return `status.${normalizeState(state)}.description`
}
