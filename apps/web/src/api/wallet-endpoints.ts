import { requestJson } from './client'
import type { TokenAnswer } from './endpoints'

// Wallet login, committee, content upload and job publication. Types mirror services/api.

export interface WalletChallenge {
  nonce: string
  message: string
  expires_at: string
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

/** Sends a signed event of one of the user's jobs (its cancellation). The node checks the proof. */
export function cancelJob(token: string, jobId: string, event: Uint8Array): Promise<{ accepted: number; pending: boolean }> {
  return requestJson(`/v1/jobs/${encodeURIComponent(jobId)}/events`, {
    method: 'POST',
    token,
    body: { event: toBase64(event) },
  })
}

/** What the network decides for a job (services/api/internal/policy). */
export interface NetworkTerms {
  fragments: number
  reward_per_unit: number
  verifier_reward_per_unit: number
  verifications_required: number
  lease_ms: number
  max_attempts: number
  expires_in_ms: number
}

export interface TermsAnswer {
  terms: NetworkTerms
  payments: boolean
}

/** Terms of a job for this workload and input size: the user never chooses them. */
export function networkTerms(
  token: string,
  query: { workload: string; size: number; width?: number; height?: number },
): Promise<TermsAnswer> {
  const params = new URLSearchParams({ workload: query.workload, size: String(query.size) })
  if (query.width) params.set('width', String(query.width))
  if (query.height) params.set('height', String(query.height))
  return requestJson(`/v1/network/terms?${params.toString()}`, { token })
}
