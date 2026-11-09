// Dépôt de l'escrow depuis le wallet du requester : une transaction `Open` que le wallet signe,
// puis envoyée et confirmée. Le site ne voit jamais la clé : seule la signature est demandée.

import { Connection, PublicKey, Transaction } from '@solana/web3.js'
import { escrowConfig, DEVNET_RPC } from './config'
import { accountSpace, maxPayout, openInstruction, type EscrowSpec } from './escrow'
import { signTransactionWithWallet, WalletFailure } from '@/platform/wallet'
import { findWorkload } from '@/workloads/find'

/** Réserve pour les frais de la transaction, en lamports. */
const FEE_RESERVE = 10_000n

export interface DepositPlan {
  deposit: bigint
  rent: bigint
  /** Ce que le requester doit avoir : loyer du compte, dépôt et frais. */
  needed: bigint
}

/** Montants du dépôt, lus sur la chaîne (loyer minimum du compte) ; sans rien signer. */
export async function planDeposit(connection: Connection, spec: EscrowSpec): Promise<DepositPlan> {
  if (!findWorkload(spec.workload)?.depositSupported) throw new WalletFailure('deposit_unsupported')
  const deposit = maxPayout(spec)
  const rent = BigInt(await connection.getMinimumBalanceForRentExemption(accountSpace(spec)))
  return { deposit, rent, needed: rent + deposit + FEE_RESERVE }
}

/** Ouvre l'escrow du job : le wallet signe, le réseau confirme. Lève WalletFailure sinon. */
export async function depositEscrow(spec: EscrowSpec, jobId: Uint8Array): Promise<void> {
  const { rpcUrl, program } = escrowConfig()
  const connection = new Connection(rpcUrl || DEVNET_RPC, 'confirmed')
  const requester = new PublicKey(spec.requester)
  const plan = await planDeposit(connection, spec)
  const balance = BigInt(await connection.getBalance(requester, 'confirmed'))
  if (balance < plan.needed) throw new WalletFailure('no_funds')

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed')
  const transaction = new Transaction({ feePayer: requester, recentBlockhash: blockhash }).add(
    openInstruction(program, jobId, spec, plan.deposit),
  )
  const signed = await signTransactionWithWallet(transaction)
  try {
    const signature = await connection.sendRawTransaction(signed.serialize())
    await waitConfirmed(connection, signature, lastValidBlockHeight)
  } catch (failure) {
    if (failure instanceof WalletFailure) throw failure
    // Erreur de la bibliothèque ou du RPC (simulation refusée, blockhash expiré...) : un message
    // précis plutôt que l'erreur générique, et le détail reste dans la console.
    console.error('[deposit] transaction failed:', failure)
    throw new WalletFailure('deposit_failed')
  }
}

/** Attend la confirmation par interrogation HTTP (pas de websocket : la CSP ne l'autorise pas). */
export async function waitConfirmed(
  connection: Connection,
  signature: string,
  lastValidBlockHeight: number,
  timeoutMs = 90_000,
): Promise<void> {
  const started = Date.now()
  for (;;) {
    const { value } = await connection.getSignatureStatuses([signature], { searchTransactionHistory: true })
    const status = value[0]
    if (status?.err) {
      console.error('[deposit] escrow refused:', status.err)
      throw new WalletFailure('deposit_failed')
    }
    if (status && (status.confirmationStatus === 'confirmed' || status.confirmationStatus === 'finalized')) {
      return
    }
    const height = await connection.getBlockHeight('confirmed')
    if (height > lastValidBlockHeight || Date.now() - started > timeoutMs) {
      throw new WalletFailure('deposit_failed')
    }
    await new Promise((resolve) => setTimeout(resolve, 1_000))
  }
}
