import { describe, expect, it } from 'vitest'
import { finalizedCount, formatElapsed, isLiveState } from './status-progress'

describe('formatElapsed', () => {
  it('reads minutes and seconds under an hour', () => {
    expect(formatElapsed(0)).toBe('0:00')
    expect(formatElapsed(4_999)).toBe('0:04')
    expect(formatElapsed(4 * 60_000 + 52_000)).toBe('4:52')
  })

  it('shows hours from one hour on', () => {
    expect(formatElapsed(3_600_000)).toBe('1:00:00')
    expect(formatElapsed(3_725_000)).toBe('1:02:05')
  })

  it('reads a negative duration as zero', () => {
    expect(formatElapsed(-5_000)).toBe('0:00')
  })
})

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
