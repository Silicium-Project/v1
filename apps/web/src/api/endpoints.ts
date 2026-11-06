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

export interface Device {
  id: string
  public_key: string
  label: string
  linked_at: string
  revoked_at?: string
}

export interface JobSummary {
  id: string
  label: string
  created_at: string
}

export type JobStateName = 'running' | 'finalized' | 'cancelled' | 'expired' | 'not_on_node' | 'unknown'

export interface JobStatus {
  state: string
  tasks?: Record<string, number>
  note?: string
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

export function revokeDevice(token: string, id: string): Promise<void> {
  return requestJson(`/v1/devices/${encodeURIComponent(id)}`, { method: 'DELETE', token })
}

export function listJobs(token: string): Promise<{ jobs: JobSummary[] }> {
  return requestJson('/v1/jobs', { token })
}

export function getJob(token: string, id: string): Promise<{ job: JobSummary; status: JobStatus }> {
  return requestJson(`/v1/jobs/${encodeURIComponent(id)}`, { token })
}
