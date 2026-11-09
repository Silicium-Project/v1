// Escrow on-chain du job : montant à déposer, compte d'escrow et instruction `Open`. Les règles
// reprennent `crates/sil-chain-core` (units, terms, wire) : les vecteurs partagés figent les octets.

import { Buffer } from 'buffer'
import { PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js'
import { BincodeWriter } from '@/signing/bincode'
import { findWorkload } from '@/workloads/find'

/** Graine du PDA de l'escrow : `[ESCROW_SEED, job_id]`. */
export const ESCROW_SEED = new TextEncoder().encode('escrow')
/** Frais de règlement par fragment, prélevés sur le dépôt (sil-chain-core::terms). */
export const SETTLEMENT_FEE = 10_000n

const HEADER_BYTES = 1536
const RECORD_BASE = 160
const RECORD_PER_APPROVER = 34
const OPEN_VARIANT = 0

/** Champs de la spec qui déterminent l'escrow (sous-ensemble de `JobSpecInput`). */
export interface EscrowSpec {
  requester: Uint8Array
  workload: string
  inputSize: number
  width: number
  height: number
  fragments: number
  verificationsRequired: number
  rewardPerUnit: number
  verifierRewardPerUnit: number
  expiresAtMs: number
}

/**
 * Unités d'un fragment en octets (unité `bytes_input`) : même découpage que `sil-wl-sha256`, les
 * premiers fragments ayant un octet de plus.
 */
export function sha256Units(inputSize: number, fragments: number, index: number): bigint {
  const count = BigInt(fragments)
  const size = BigInt(inputSize)
  if (count > (size > 0n ? size : 1n)) {
    throw new Error('entrée plus petite que le nombre de fragments')
  }
  const base = size / count
  const extra = size % count
  return base + (BigInt(index) < extra ? 1n : 0n)
}

/** Budget d'un fragment : unités × (tarif claimant + vérificateurs × tarif vérificateur) + frais. */
/**
 * Unités d'un fragment d'image (unité `pixels_band`) : même découpage que `sil-wl-render` et
 * `sil-chain-core::units`. Les blocs de lignes sont répartis sur les fragments ; la dernière bande
 * est coupée à la hauteur de l'image.
 */
export function pixelBandUnits(
  width: number,
  height: number,
  fragments: number,
  index: number,
  blockRows: number,
  maxSide: number,
): bigint {
  if (width < 1 || height < 1 || width > maxSide || height > maxSide) {
    throw new Error("dimensions de l'image")
  }
  const blocks = BigInt(Math.ceil(height / blockRows))
  const count = BigInt(fragments)
  if (count > blocks) throw new Error("image plus petite que le nombre de fragments")
  const rows = BigInt(blockRows)
  const total = BigInt(height)
  const base = blocks / count
  const extra = blocks % count
  const at = BigInt(index)
  const first = at * base + (at < extra ? at : extra)
  const last = first + base + (at < extra ? 1n : 0n)
  const end = last * rows < total ? last * rows : total
  return BigInt(width) * (end - first * rows)
}

/** Unités d'un fragment, selon l'unité du workload (descripteur `contracts/workloads`). */
export function fragmentUnits(spec: EscrowSpec, index: number): bigint {
  const workload = findWorkload(spec.workload)
  if (!workload) throw new Error('workload sans descripteur')
  if (workload.unit === 'pixels_band') {
    return pixelBandUnits(spec.width, spec.height, spec.fragments, index, workload.blockRows, workload.maxSide)
  }
  return sha256Units(spec.inputSize, spec.fragments, index)
}

/** Budget d'un fragment : unités × (tarif claimant + vérificateurs × tarif vérificateur) + frais. */
export function fragmentBudget(spec: EscrowSpec, index: number): bigint {
  const perUnit =
    BigInt(spec.rewardPerUnit) + BigInt(spec.verificationsRequired) * BigInt(spec.verifierRewardPerUnit)
  return fragmentUnits(spec, index) * perUnit + SETTLEMENT_FEE
}

/** Dépôt total : somme des budgets (`EscrowTerms::max_payout`). */
export function maxPayout(spec: EscrowSpec): bigint {
  if (!findWorkload(spec.workload)) {
    throw new Error('dépôt non disponible pour ce workload')
  }
  let total = 0n
  for (let index = 0; index < spec.fragments; index++) {
    total += fragmentBudget(spec, index)
  }
  return total
}

/** Taille du compte d'escrow en octets (`sil-chain-core` / `chain/program` : `account_space`). */
export function accountSpace(spec: EscrowSpec): number {
  const record = RECORD_BASE + RECORD_PER_APPROVER * spec.verificationsRequired
  return HEADER_BYTES + record * spec.fragments + 4
}

/** Adresse du compte d'escrow d'un job (PDA). */
export function escrowAddress(program: PublicKey, jobId: Uint8Array): PublicKey {
  return PublicKey.findProgramAddressSync([ESCROW_SEED, jobId], program)[0]
}

/** Données de l'instruction `Open` : variante, identifiant, termes, dépôt (bincode 1). */
export function openData(jobId: Uint8Array, spec: EscrowSpec, deposit: bigint): Uint8Array {
  const writer = new BincodeWriter().u32(OPEN_VARIANT).bytes(jobId)
  writer.bytes(spec.requester)
  writer
    .string(spec.workload)
    .u64(spec.inputSize)
    .u32(spec.width)
    .u32(spec.height)
    .u32(spec.fragments)
    .u32(spec.verificationsRequired)
    .u64(spec.rewardPerUnit)
    .u64(spec.verifierRewardPerUnit)
    .u64(spec.expiresAtMs)
  return writer.u64(deposit).finish()
}

/** Instruction `Open` : le requester signe, paie le loyer du compte et le dépôt. */
export function openInstruction(
  program: PublicKey,
  jobId: Uint8Array,
  spec: EscrowSpec,
  deposit: bigint,
): TransactionInstruction {
  const requester = new PublicKey(spec.requester)
  return new TransactionInstruction({
    programId: program,
    keys: [
      { pubkey: requester, isSigner: true, isWritable: true },
      { pubkey: escrowAddress(program, jobId), isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.from(openData(jobId, spec, deposit)),
  })
}
