// Réglages du dépôt on-chain. Devnet par défaut ; surchargés par VITE_SOLANA_RPC et VITE_ESCROW_PROGRAM.
import { PublicKey } from '@solana/web3.js'

export const DEVNET_RPC = 'https://api.devnet.solana.com'
/** Programme d'escrow déployé sur devnet (chain/README.md). */
export const DEVNET_PROGRAM = '5nvVoget71j65HtMZmfVJjnwSePDtrG5hzAPpjMbrMkx'

export function escrowConfig(): { rpcUrl: string; program: PublicKey } {
  const env = import.meta.env
  const rpcUrl = (env.VITE_SOLANA_RPC as string | undefined) || DEVNET_RPC
  const program = new PublicKey((env.VITE_ESCROW_PROGRAM as string | undefined) || DEVNET_PROGRAM)
  return { rpcUrl, program }
}
