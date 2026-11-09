// @vitest-environment node
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { PublicKey } from '@solana/web3.js'
import { toHex } from '../signing/cbor'
import { escrowAddress, fragmentBudget, maxPayout, openData, pixelBandUnits, sha256Units, SETTLEMENT_FEE, type EscrowSpec } from './escrow'

// Vecteur figé par crates/sil-chain-core (wire_vectors.rs) : même instruction Open, même octets.
const expected = readFileSync(resolve(process.cwd(), '../../contracts/vectors/escrow-open.hex'), 'utf8').trim()

const shared: EscrowSpec = {
  requester: new Uint8Array(32).fill(9),
  workload: 'calcul/sha256',
  inputSize: 2048,
  width: 0,
  height: 0,
  fragments: 2,
  verificationsRequired: 1,
  rewardPerUnit: 100,
  verifierRewardPerUnit: 10,
  expiresAtMs: 1_800_000_000_000,
}

describe('escrow Open instruction', () => {
  it('encodes the same bytes as the Rust program interface', () => {
    expect(toHex(openData(new Uint8Array(32).fill(7), shared, 245_280n))).toBe(expected)
  })
})

describe('sha256 units and deposit', () => {
  it('splits bytes like sil-wl-sha256: the first remainder fragments get one more', () => {
    expect([0, 1, 2].map((i) => sha256Units(10, 3, i))).toEqual([4n, 3n, 3n])
  })

  it('refuses an input smaller than the fragment count', () => {
    expect(() => sha256Units(1, 2, 0)).toThrow()
  })

  it('budgets units × (reward + verifier reward) plus the settlement fee, and the deposit is their sum', () => {
    expect(fragmentBudget(shared, 0)).toBe(122_640n)
    expect(maxPayout(shared)).toBe(245_280n)
  })

  it('refuses a workload the network does not describe', () => {
    expect(() => maxPayout({ ...shared, workload: 'ia/llm' })).toThrow()
  })
})

describe('pixel band units and deposit (rendu/raytracer)', () => {
  // Mêmes valeurs que les tests de sil-chain-core::units : bandes de 8 lignes, 16 384 px au plus.
  const band = (width: number, height: number, fragments: number) =>
    Array.from({ length: fragments }, (_, i) => pixelBandUnits(width, height, fragments, i, 8, 16_384))

  it('splits the blocks of rows like sil-wl-render', () => {
    expect(band(16, 24, 2)).toEqual([256n, 128n])
  })

  it('cuts the last band at the image height', () => {
    expect(band(10, 20, 3)).toEqual([80n, 80n, 40n])
  })

  it('always adds up to the pixels of the image', () => {
    for (let height = 1; height <= 40; height += 1)
      for (const width of [1, 7, 64])
        for (let fragments = 1; fragments <= Math.ceil(height / 8); fragments += 1)
          expect(band(width, height, fragments).reduce((sum, value) => sum + value, 0n)).toBe(
            BigInt(width * height),
          )
  })

  it('refuses an image with fewer blocks than fragments', () => {
    expect(() => pixelBandUnits(16, 8, 2, 0, 8, 16_384)).toThrow()
  })

  it('deposits the units of the image with the same per-unit rates as the network', () => {
    const image: EscrowSpec = { ...shared, workload: 'rendu/raytracer', width: 16, height: 24, fragments: 2, rewardPerUnit: 1, verifierRewardPerUnit: 1 }
    // Unités 256 et 128 ; budget = unités × (1 + 1) + frais de règlement.
    expect(fragmentBudget(image, 0)).toBe(256n * 2n + SETTLEMENT_FEE)
    expect(maxPayout(image)).toBe((256n + 128n) * 2n + 2n * SETTLEMENT_FEE)
  })
})

describe('escrow address', () => {
  it('is derived from the program and the job id, stable for the same job', () => {
    const program = new Uint8Array(32).fill(5)
    const a = escrowAddress(new PublicKey(program), new Uint8Array(32).fill(7))
    const b = escrowAddress(new PublicKey(program), new Uint8Array(32).fill(7))
    expect(a.toBase58()).toBe(b.toBase58())
  })
})
