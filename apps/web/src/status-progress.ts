import type { JobStateName } from '@/api/endpoints'

/** Refresh period, in milliseconds, while a job is still moving. */
export const POLL_MS = 5000

/** A job is still moving while it runs; `unknown` may be a node that is only briefly unreachable. */
export function isLiveState(state: JobStateName): boolean {
  return state === 'running' || state === 'unknown'
}

/** Fragments already final in the ledger, out of the fragments the node reports. */
export function finalizedCount(fragments: readonly { finalized: boolean }[]): number {
  return fragments.filter((fragment) => fragment.finalized).length
}
