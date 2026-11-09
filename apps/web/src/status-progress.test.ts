import { describe, expect, it } from 'vitest'
import { finalizedCount, isLiveState } from './status-progress'

describe('isLiveState', () => {
  it('keeps polling while the job runs or its state is unknown', () => {
    expect(isLiveState('running')).toBe(true)
    expect(isLiveState('unknown')).toBe(true)
  })

  it('stops once the job is final, cancelled, expired or unknown to the node', () => {
    expect(isLiveState('finalized')).toBe(false)
    expect(isLiveState('cancelled')).toBe(false)
    expect(isLiveState('expired')).toBe(false)
    expect(isLiveState('not_on_node')).toBe(false)
  })
})

describe('finalizedCount', () => {
  it('counts the fragments that have a final result', () => {
    expect(finalizedCount([{ finalized: true }, { finalized: false }, { finalized: true }])).toBe(2)
    expect(finalizedCount([])).toBe(0)
  })
})
