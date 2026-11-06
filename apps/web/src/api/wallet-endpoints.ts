import { requestJson } from './client'
import type { TokenAnswer } from './endpoints'

// Wallet login, committee, content upload and job publication. Types mirror services/api.

export interface WalletChallenge {
  nonce: string
  message: string
  expires_at: string
}

export interface CommitteeAnswer {
  committee: string[] // coordinator keys, 64 lower-case hex each
  threshold: number
  workloads: string[]
  payments: boolean // false until the Solana escrow adapter exists
}

export interface ContentReceipt {
  content_id: string
  size: number
}

export interface PublishedJob {
  id: string
  label: string
  created_at: string
}

/** Base64 (standard alphabet), the form the API expects for signed events. */
export function toBase64(bytes: Uint8Array): string {
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}

export function walletChallenge(address: string): Promise<WalletChallenge> {
  return requestJson('/v1/auth/wallet/challenge', { method: 'POST', body: { address } })
}

export function walletLogin(address: string, nonce: string, signature: Uint8Array): Promise<TokenAnswer> {
  return requestJson('/v1/auth/wallet/login', {
    method: 'POST',
    body: { address, nonce, signature: toBase64(signature) },
  })
}

export function deleteChallenge(token: string): Promise<WalletChallenge> {
  return requestJson('/v1/account/delete-challenge', { method: 'POST', token })
}

export function deleteAccountWithWallet(token: string, nonce: string, signature: Uint8Array): Promise<void> {
  return requestJson('/v1/account', {
    method: 'DELETE',
    token,
    body: { nonce, signature: toBase64(signature) },
  })
}

export function committee(token: string): Promise<CommitteeAnswer> {
  return requestJson('/v1/network/committee', { token })
}

export function putContent(token: string, data: Uint8Array): Promise<ContentReceipt> {
  return requestJson('/v1/content', { method: 'PUT', token, rawBody: data })
}

export function publishJob(token: string, jobId: string, event: Uint8Array, label: string): Promise<PublishedJob> {
  return requestJson('/v1/jobs', {
    method: 'POST',
    token,
    body: { job_id: jobId, event: toBase64(event), label },
  })
}
