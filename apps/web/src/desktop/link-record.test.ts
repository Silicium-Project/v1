import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Device } from '@/api/endpoints'
import { isDesktop, machineKey, saveLink, savedLink } from '@/platform/desktop'
import { currentLinkRecord, saveLinkRecord } from './link-record'

vi.mock('@/platform/desktop', () => ({
  isDesktop: vi.fn(),
  machineKey: vi.fn(),
  savedLink: vi.fn(),
  saveLink: vi.fn(),
}))

const device = {
  id: 'dev-1',
  public_key: 'aa',
  label: 'PC bureau',
  linked_at: '2026-11-08T10:00:00Z',
} as Device

const saved = {
  device_id: 'dev-1',
  label: 'PC bureau',
  linked_at: '2026-11-08T10:00:00Z',
  public_key: 'aa',
}

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
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', memoryStorage())
    vi.mocked(isDesktop).mockReturnValue(true)
    vi.mocked(machineKey).mockResolvedValue('aa')
    vi.mocked(savedLink).mockResolvedValue(null)
    vi.mocked(saveLink).mockResolvedValue(undefined)
  })

  it('reads the link saved by the desktop shell', async () => {
    vi.mocked(savedLink).mockResolvedValue(saved)
    await expect(currentLinkRecord()).resolves.toEqual(saved)
  })

  it('reads nothing outside the desktop application', async () => {
    vi.mocked(isDesktop).mockReturnValue(false)
    await expect(currentLinkRecord()).resolves.toBeNull()
  })

  it('reads nothing when no link was saved', async () => {
    await expect(currentLinkRecord()).resolves.toBeNull()
  })

  it('reads nothing when the shell refuses to answer', async () => {
    vi.mocked(savedLink).mockRejectedValue(new Error('unavailable'))
    await expect(currentLinkRecord()).resolves.toBeNull()
  })

  it('moves a legacy browser-storage link to the shell once, when the key matches', async () => {
    globalThis.localStorage.setItem('silicium.desktop.link', JSON.stringify(saved))
    await expect(currentLinkRecord()).resolves.toEqual(saved)
    expect(saveLink).toHaveBeenCalledWith(saved)
  })

  it('ignores a legacy link made with another machine key', async () => {
    globalThis.localStorage.setItem(
      'silicium.desktop.link',
      JSON.stringify({ ...saved, public_key: 'bb' }),
    )
    await expect(currentLinkRecord()).resolves.toBeNull()
    expect(saveLink).not.toHaveBeenCalled()
  })

  it('ignores a malformed legacy record', async () => {
    globalThis.localStorage.setItem('silicium.desktop.link', '{"label":"x"}')
    await expect(currentLinkRecord()).resolves.toBeNull()
  })

  it('saves the link with the device fields and the machine key', async () => {
    await saveLinkRecord(device, 'aa')
    expect(saveLink).toHaveBeenCalledWith(saved)
  })

  it('does not throw when the shell cannot save the link', async () => {
    vi.mocked(saveLink).mockRejectedValue(new Error('storage'))
    await expect(saveLinkRecord(device, 'aa')).resolves.toBeUndefined()
  })
})
