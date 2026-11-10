import type { JobStateName } from '@/api/endpoints'

/** Refresh period, in milliseconds, while a job is still moving. */
export const POLL_MS = 5000

/** A job is still moving while it runs; `unknown` may be a node that is only briefly unreachable. */
export function isLiveState(state: JobStateName): boolean {
  return state === 'running' || state === 'unknown'
}

/** Elapsed time as m:ss, or h:mm:ss from one hour on. Negative values read as zero. */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = total % 60
  const pad = (value: number): string => String(value).padStart(2, '0')
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${minutes}:${pad(seconds)}`
}

/** Fragments already final in the ledger, out of the fragments the node reports. */
export function finalizedCount(fragments: readonly { finalized: boolean }[]): number {
  return fragments.filter((fragment) => fragment.finalized).length
}
