// Solana wallet bridge (Phantom or any wallet that injects the Solana provider API).
// The wallet holds the requester key: the site never sees it. Every call here needs the
// user to approve in the wallet; a refusal is reported as wallet_rejected.

export type WalletCode =
  | 'no_wallet'
  | 'wallet_rejected'
  | 'wallet_key'
  | 'no_funds'
  | 'deposit_failed'
  | 'deposit_unsupported'

export class WalletFailure extends Error {
  constructor(readonly code: WalletCode) {
    super(code)
    this.name = 'WalletFailure'
  }
}

interface PublicKeyLike {
  toBytes(): Uint8Array
}

interface SolanaProvider {
  connect(): Promise<{ publicKey: PublicKeyLike }>
  signMessage(message: Uint8Array, display?: 'utf8' | 'hex'): Promise<{ signature: Uint8Array }>
  signTransaction?<T>(transaction: T): Promise<T>
}

/** The injected provider, or null when no Solana wallet is installed. */
export function walletProvider(): SolanaProvider | null {
  if (typeof window === 'undefined') return null
  const w = window as unknown as { solana?: Partial<SolanaProvider>; phantom?: { solana?: Partial<SolanaProvider> } }
  const candidate = w.phantom?.solana ?? w.solana
  if (candidate && typeof candidate.connect === 'function' && typeof candidate.signMessage === 'function') {
    return candidate as SolanaProvider
  }
  return null
}

/** Asks the wallet for the account and returns its 32-byte public key. */
export async function connectWallet(): Promise<Uint8Array> {
  const provider = walletProvider()
  if (!provider) throw new WalletFailure('no_wallet')
  try {
    const { publicKey } = await provider.connect()
    const key = publicKey.toBytes()
    if (key.length !== 32) throw new WalletFailure('wallet_key')
    return key
  } catch (failure) {
    if (failure instanceof WalletFailure) throw failure
    console.error('[wallet] connect failed:', failure)
    throw new WalletFailure('wallet_rejected')
  }
}

/** Asks the wallet to sign the exact bytes; returns the 64-byte Ed25519 signature. */
export async function signWithWallet(message: Uint8Array, display: 'utf8' | 'hex' = 'utf8'): Promise<Uint8Array> {
  const provider = walletProvider()
  if (!provider) throw new WalletFailure('no_wallet')
  try {
    const { signature } = await provider.signMessage(message, display)
    if (signature.length !== 64) throw new WalletFailure('wallet_key')
    return signature
  } catch (failure) {
    if (failure instanceof WalletFailure) throw failure
    console.error('[wallet] signMessage failed:', failure)
    throw new WalletFailure('wallet_rejected')
  }
}

/** Asks the wallet to sign a transaction (the requester approves the escrow deposit). */
export async function signTransactionWithWallet<T>(transaction: T): Promise<T> {
  const provider = walletProvider()
  if (!provider?.signTransaction) throw new WalletFailure('no_wallet')
  try {
    return await provider.signTransaction(transaction)
  } catch (failure) {
    console.error('[wallet] signTransaction failed:', failure)
    throw new WalletFailure('wallet_rejected')
  }
}
