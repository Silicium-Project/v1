import { afterEach, describe, expect, it, vi } from 'vitest'
import { DesktopFailure, isDesktop, machineKey, signLinkCode } from './desktop'

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})

describe('desktop bridge', () => {
  it('is absent in a browser, where every call is refused as unavailable', async () => {
    expect(isDesktop()).toBe(false)
    await expect(machineKey()).rejects.toMatchObject({ code: 'unavailable' })
  })

  it('forwards commands to the shell when it is present', async () => {
    const invoke = vi.fn(async () => 'abcd')
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = { invoke }
    expect(isDesktop()).toBe(true)
    await expect(machineKey()).resolves.toBe('abcd')
    expect(invoke).toHaveBeenCalledWith('machine_key', undefined)
  })

  it('maps a stable error string from the shell to its code', async () => {
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      invoke: async () => {
        throw 'bad_code'
      },
    }
    const error = await signLinkCode('x').catch((e: unknown) => e)
    expect(error).toBeInstanceOf(DesktopFailure)
    expect(error).toMatchObject({ code: 'bad_code' })
  })

  it('treats an unknown shell error as unavailable, never as a guess', async () => {
    ;(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      invoke: async () => {
        throw new Error('C:\Users\secret\device.key')
      },
    }
    await expect(machineKey()).rejects.toMatchObject({ code: 'unavailable' })
  })
})
