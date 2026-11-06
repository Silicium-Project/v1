import { cbor, toHex } from './cbor'

// Builds what a JobPublished needs: the spec and the event, as sil-ledger encodes them, and the
// texts the requester's wallet signs. Phantom refuses binary messages that look like Solana
// transactions, so the signed texts are plain ASCII lines in a fixed order
// (sil-ledger src/wallet_text.rs). Both sides must produce the same text, byte for byte.

export interface JobSpecInput {
  requester: Uint8Array // 32 bytes, Ed25519 public key (wallet key)
  nonce: number
  workload: string // e.g. "calcul/sha256"
  input: Uint8Array // 32 bytes, content id of the input
  inputSize: number // bytes of the input: the workload turns it into units
  width: number // image width in pixels (rendu/blocs), 0 for a workload without image
  height: number // image height in pixels (rendu/blocs), 0 for a workload without image
  fragments: number
  committee: Uint8Array[] // 32-byte keys, at most 7
  threshold: number
  verificationsRequired: number
  leaseMs: number
  maxAttempts: number
  rewardPerUnit: number // paid to the claimant, for each unit of its fragment
  verifierRewardPerUnit: number // paid to each verifier, for each unit of the fragment
  expiresAtMs: number
}

export interface EventInput {
  requester: Uint8Array
  job: Uint8Array // 32 bytes: SHA-256 of domainFrame('sil/v1/job-id', spec CBOR)
  sequence: number // 1 for the first event of a requester chain
  previous: Uint8Array // 32 zero bytes for the first event
  issuedAtMs: number
}

/** Field order is the struct order in sil-ledger (JobSpec). Do not reorder. */
export function encodeJobSpec(spec: JobSpecInput): Uint8Array {
  return cbor.map([
    ['version', cbor.uint(1)],
    ['requester', cbor.bytes(spec.requester)],
    ['nonce', cbor.uint(spec.nonce)],
    ['workload', cbor.text(spec.workload)],
    ['input', cbor.bytes(spec.input)],
    ['input_size', cbor.uint(spec.inputSize)],
    ['width', cbor.uint(spec.width)],
    ['height', cbor.uint(spec.height)],
    ['fragments', cbor.uint(spec.fragments)],
    ['committee', cbor.array(spec.committee.map((key) => cbor.bytes(key)))],
    ['threshold', cbor.uint(spec.threshold)],
    ['verifications_required', cbor.uint(spec.verificationsRequired)],
    ['lease_ms', cbor.uint(spec.leaseMs)],
    ['max_attempts', cbor.uint(spec.maxAttempts)],
    ['reward_per_unit', cbor.uint(spec.rewardPerUnit)],
    ['verifier_reward_per_unit', cbor.uint(spec.verifierRewardPerUnit)],
    ['expires_at_ms', cbor.uint(spec.expiresAtMs)],
  ])
}

/** The text the requester signs for the spec: one line per field, fixed order. */
export function specText(spec: JobSpecInput): string {
  return [
    'Silicium JobSpec v1',
    `Requester: ${toHex(spec.requester)}`,
    `Nonce: ${spec.nonce}`,
    `Workload: ${spec.workload}`,
    `Input: ${toHex(spec.input)}`,
    `Input size: ${spec.inputSize}`,
    `Width: ${spec.width}`,
    `Height: ${spec.height}`,
    `Fragments: ${spec.fragments}`,
    `Committee: ${spec.committee.map(toHex).join(',')}`,
    `Threshold: ${spec.threshold}`,
    `Verifications: ${spec.verificationsRequired}`,
    `Lease ms: ${spec.leaseMs}`,
    `Max attempts: ${spec.maxAttempts}`,
    `Reward per unit: ${spec.rewardPerUnit}`,
    `Verifier reward per unit: ${spec.verifierRewardPerUnit}`,
    `Expires ms: ${spec.expiresAtMs}`,
  ].join('\n')
}

/** Bytes the wallet signs for the spec (UTF-8 of specText). */
export function specSigningMessage(spec: JobSpecInput): Uint8Array {
  return new TextEncoder().encode(specText(spec))
}

/** The text the requester signs for its JobPublished event; it binds the spec signature. */
export function jobPublishedText(event: EventInput, specSignature: Uint8Array): string {
  return [
    'Silicium Event v1',
    `Author: ${toHex(event.requester)}`,
    `Job: ${toHex(event.job)}`,
    `Sequence: ${event.sequence}`,
    `Previous: ${toHex(event.previous)}`,
    `Issued ms: ${event.issuedAtMs}`,
    'Body: JobPublished',
    `Spec signature: ${toHex(specSignature)}`,
  ].join('\n')
}

/** Bytes the wallet signs for the event (UTF-8 of jobPublishedText). */
export function eventSigningMessage(event: EventInput, specSignature: Uint8Array): Uint8Array {
  return new TextEncoder().encode(jobPublishedText(event, specSignature))
}

/** Event fields that the node stores (sil-ledger Event Core, same order). */
export function encodeEventCore(event: EventInput, body: Uint8Array): Uint8Array {
  return cbor.map([
    ['version', cbor.uint(1)],
    ['author', cbor.bytes(event.requester)],
    ['job', cbor.bytes(event.job)],
    ['sequence', cbor.uint(event.sequence)],
    ['previous', cbor.bytes(event.previous)],
    ['issued_at_ms', cbor.uint(event.issuedAtMs)],
    ['body', body],
  ])
}

/** EventBody::JobPublished { spec } is an externally tagged enum: {"JobPublished": {"spec": ...}}. */
export function encodeJobPublishedBody(spec: JobSpecInput, specSignature: Uint8Array): Uint8Array {
  const signedSpec = cbor.map([
    ['spec', encodeJobSpec(spec)],
    ['signature', cbor.bytes(specSignature)],
  ])
  return cbor.map([['JobPublished', cbor.map([['spec', signedSpec]])]])
}

/** Full signed event, as sent to POST /v1/jobs (base64) and stored by the node. */
export function encodeSignedEvent(event: EventInput, body: Uint8Array, signature: Uint8Array): Uint8Array {
  return cbor.map([
    ['version', cbor.uint(1)],
    ['author', cbor.bytes(event.requester)],
    ['job', cbor.bytes(event.job)],
    ['sequence', cbor.uint(event.sequence)],
    ['previous', cbor.bytes(event.previous)],
    ['issued_at_ms', cbor.uint(event.issuedAtMs)],
    ['body', body],
    ['signature', cbor.bytes(signature)],
  ])
}
