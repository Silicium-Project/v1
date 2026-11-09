// @vitest-environment node
import type { Connection } from '@solana/web3.js'
import { describe, expect, it } from 'vitest'
import { waitConfirmed } from './deposit'

// Une connexion factice : pas de statut au premier tour, puis confirmé. Aucune requête réseau.
function fakeConnection(statuses: Array<{ confirmationStatus: string } | null>) {
  let calls = 0
  return {
    async getSignatureStatuses() {
      const value = [statuses[Math.min(calls++, statuses.length - 1)]]
      return { value }
    },
    async getBlockHeight() {
      return 10
    },
  } as unknown as Connection
}

describe('waitConfirmed', () => {
  it('returns once the signature is confirmed, polling over HTTP', async () => {
    const connection = fakeConnection([null, { confirmationStatus: 'confirmed' }])
    await expect(waitConfirmed(connection, 'sig', 100)).resolves.toBeUndefined()
  })

  it('gives up when the blockhash expires before confirmation', async () => {
    const connection = fakeConnection([null])
    await expect(waitConfirmed(connection, 'sig', 5)).rejects.toMatchObject({ code: 'deposit_failed' })
  })
})
