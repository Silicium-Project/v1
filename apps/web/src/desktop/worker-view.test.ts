import { describe, expect, it } from 'vitest'
import { powerPercent, reputationText, stageKey } from './worker-view'

describe('powerPercent', () => {
  it('is the share of allowed tasks in use', () => {
    expect(powerPercent(1, 2)).toBe(50)
    expect(powerPercent(0, 4)).toBe(0)
  })

  it('is zero when the reputation allows nothing', () => {
    expect(powerPercent(3, 0)).toBe(0)
  })

  it('never exceeds 100', () => {
    expect(powerPercent(5, 2)).toBe(100)
  })
})

describe('stageKey', () => {
  it('tells a task being computed from one waiting for verification', () => {
    expect(stageKey({ status: 'claimed', submitted: false })).toBe('computing')
    expect(stageKey({ status: 'claimed', submitted: true })).toBe('awaiting')
    expect(stageKey({ status: 'completed', submitted: true })).toBe('awaiting')
  })

  it('names verified and disputed tasks', () => {
    expect(stageKey({ status: 'verified', submitted: true })).toBe('verified')
    expect(stageKey({ status: 'disputed', submitted: true })).toBe('disputed')
  })

  it('falls back to other for any status this view does not show', () => {
    expect(stageKey({ status: 'lease_expired', submitted: false })).toBe('other')
  })
})

describe('reputationText', () => {
  it('shows two decimals', () => {
    expect(reputationText(0.5)).toBe('0.50')
    expect(reputationText(0.8333)).toBe('0.83')
  })
})
