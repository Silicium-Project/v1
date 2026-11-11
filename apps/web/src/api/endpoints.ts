import { requestBlob, requestJson } from './client'

// Types mirror the JSON written by services/api (internal/server, internal/jobs).

export interface TokenAnswer {
  token: string
  expires_at: string
}

export interface Profile {
  id: string
  email: string // empty for a wallet account
  address: string // base58 wallet address, empty for an e-mail account
  created_at: string
}

export interface LinkCode {
  code: string
  expires_at: string
}

export interface DeviceTelemetry {
  observed_at: string
  received_at: string
  version: string
  cpu_pct: number
  mem_pct: number
  cpu_cores: number
  compute: boolean
  verify: boolean
  tasks_active: number
  tasks_in_progress: number
  tasks_finalized: number
  tasks_other: number
  reputation_score?: number | null
  reputation_good?: number | null
  reputation_bad?: number | null
  worker_lamports?: number
  verifier_lamports?: number
  fragments_settled?: number
}

export interface Device {
  id: string
  public_key: string
  label: string
  linked_at: string
  revoked_at?: string
  telemetry?: DeviceTelemetry
}

export interface JobSummary {
  id: string
  label: string
  created_at: string
}

export type JobStateName = 'running' | 'finalized' | 'cancelled' | 'expired' | 'not_on_node' | 'unknown'

export interface JobStatus {
  state: string
  /** Workload id as the node knows it, and whether it produces a picture. */
  workload?: string
  image?: boolean
  /** Last event of the requester chain: the next event (a cancellation) follows it. */
  requester_head?: { sequence: number; hash: string }
  tasks?: Record<string, number>
  /** Progress of each fragment, by index, as the node reports it. */
  fragments?: FragmentStatus[]
  note?: string
}

export interface FragmentStatus {
  index: number
  /** Node task status name: Open, Claimed, Completed, Verified, Finalized… */
  status: string
  attempts: number
  approvals: number
  finalized: boolean
}

export function login(email: string, password: string): Promise<TokenAnswer> {
  return requestJson('/v1/auth/login', { method: 'POST', body: { email, password } })
}

export function register(email: string, password: string): Promise<{ id: string; email: string }> {
  return requestJson('/v1/auth/register', { method: 'POST', body: { email, password } })
}

export function logout(token: string): Promise<void> {
  return requestJson('/v1/auth/logout', { method: 'POST', token })
}

export function profile(token: string): Promise<Profile> {
  return requestJson('/v1/account', { token })
}

export function exportAccount(token: string): Promise<Blob> {
  return requestBlob('/v1/account/export', token)
}

export interface CredentialsChange {
  email: string
  current_password?: string
  new_password?: string
}

/** Adds an e-mail and a password to a wallet account, or changes them. Returns the new profile. */
export function setCredentials(token: string, change: CredentialsChange): Promise<Profile> {
  return requestJson('/v1/account/credentials', { method: 'PUT', token, body: change })
}

/** The finished result of a job: a PNG picture for a render job, a text file otherwise. Owner only. */
export function jobResult(token: string, id: string): Promise<Blob> {
  return requestBlob(`/v1/jobs/${encodeURIComponent(id)}/result`, token)
}

export function deleteAccount(token: string, password: string): Promise<void> {
  return requestJson('/v1/account', { method: 'DELETE', token, body: { password } })
}

export function newLinkCode(token: string): Promise<LinkCode> {
  return requestJson('/v1/devices/link-codes', { method: 'POST', token })
}

export function listDevices(token: string): Promise<{ devices: Device[] }> {
  return requestJson('/v1/devices', { token })
}

export function linkDevice(
  code: string,
  publicKey: string,
  signature: string,
  label: string,
): Promise<Device> {
  return requestJson('/v1/devices/link', {
    method: 'POST',
    body: { code, public_key: publicKey, signature, label },
  })
}

/** Who a link code belongs to, before the machine links to it. No session: the code is the input. */
export interface LinkPreview {
  account: string
  kind: 'email' | 'wallet'
  expires_at: string
}

export function previewLinkCode(code: string): Promise<LinkPreview> {
  return requestJson(`/v1/devices/link-codes/${encodeURIComponent(code)}`)
}

export function revokeDevice(token: string, id: string): Promise<void> {
  return requestJson(`/v1/devices/${encodeURIComponent(id)}`, { method: 'DELETE', token })
}

/** Public summary of the network as the node sees it: counts only, no identifier. */
export interface NetworkStatus {
  online: number
  by_role: Record<string, number>
  node_reachable: boolean
  updated_at: string
}

export function getNetworkStatus(): Promise<NetworkStatus> {
  return requestJson('/v1/network/status')
}

/** Deletes a revoked device for good. The API refuses an active one (409 device_active). */
export function purgeDevice(token: string, id: string): Promise<void> {
  return requestJson(`/v1/devices/${encodeURIComponent(id)}/purge`, { method: 'DELETE', token })
}

export function listJobs(token: string): Promise<{ jobs: JobSummary[] }> {
  return requestJson('/v1/jobs', { token })
}

export function getJob(token: string, id: string): Promise<{ job: JobSummary; status: JobStatus }> {
  return requestJson(`/v1/jobs/${encodeURIComponent(id)}`, { token })
}
