import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Device } from '@/api/endpoints'
import { readLinkRecord, saveLinkRecord } from './link-record'

const device = {
  id: 'dev-1',
  public_key: 'aa',
  label: 'PC bureau',
  linked_at: '2026-11-08T10:00:00Z',
} as Device

function memoryStorage(): Storage {
  const data = new Map<string, string>()
  return {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => {
      data.set(key, value)
    },
    removeItem: (key: string) => {
      data.delete(key)
    },
    clear: () => data.clear(),
    key: () => null,
    get length() {
      return data.size
    },
  } as unknown as Storage
}

describe('link-record', () => {
  beforeEach(() => {
    vi.stubGlobal('localStorage', memoryStorage())
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('reads back the link saved for this machine', () => {
    saveLinkRecord(device, 'aa')
    expect(readLinkRecord()).toEqual({
      device_id: 'dev-1',
      label: 'PC bureau',
      linked_at: '2026-11-08T10:00:00Z',
      public_key: 'aa',
    })
  })

  it('reads nothing when no link was saved', () => {
    expect(readLinkRecord()).toBeNull()
  })

  it('ignores a malformed record', () => {
    globalThis.localStorage.setItem('silicium.desktop.link', 'pas du json')
    expect(readLinkRecord()).toBeNull()
    globalThis.localStorage.setItem('silicium.desktop.link', '{"label":"x"}')
    expect(readLinkRecord()).toBeNull()
  })

  it('does not throw when storage is unavailable', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('denied')
      },
      setItem: () => {
        throw new Error('denied')
      },
    })
    expect(() => saveLinkRecord(device, 'aa')).not.toThrow()
    expect(readLinkRecord()).toBeNull()
  })
})
