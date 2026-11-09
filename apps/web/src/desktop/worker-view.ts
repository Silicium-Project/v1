import type { WorkerTask } from '@/platform/desktop'

/** Share of the allowed parallel tasks in use, 0 to 100. Nothing allowed means nothing used. */
export function powerPercent(active: number, allowed: number): number {
  if (allowed <= 0) return 0
  return Math.min(100, Math.round((active / allowed) * 100))
}

/** Which step a task is at, as a message key suffix (`worker.stage.<key>`). */
export function stageKey(task: Pick<WorkerTask, 'status' | 'submitted'>): 'computing' | 'awaiting' | 'verified' | 'disputed' | 'other' {
  if (task.status === 'verified') return 'verified'
  if (task.status === 'disputed') return 'disputed'
  if (task.submitted || task.status === 'completed') return 'awaiting'
  if (task.status === 'claimed') return 'computing'
  return 'other'
}

/** Reputation score on two decimals, as shown in the dashboard. */
export function reputationText(score: number): string {
  return score.toFixed(2)
}
