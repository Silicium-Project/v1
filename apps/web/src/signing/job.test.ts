// @vitest-environment node
import { createHash, createPublicKey, verify } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { domainFrame, fromHex, toHex } from './cbor'
import {
  encodeJobPublishedBody,
  encodeJobSpec,
  encodeSignedEvent,
  encodeEventCore,
  eventSigningMessage,
  jobPublishedText,
  specSigningMessage,
  specText,
  type EventInput,
  type JobSpecInput,
} from './job'

// Vectors written by sil-ledger (crates/sil-ledger/src/tests/vectors.rs). If the Rust format
// changes on purpose, regenerate them there and this test follows.
const vectors = JSON.parse(
  readFileSync(resolve(process.cwd(), '../../contracts/vectors/job-published.json'), 'utf8'),
) as Record<string, string | string[] | number>

const hex = (name: string) => fromHex(vectors[name] as string)

function specFromVector(): JobSpecInput {
  return {
    requester: hex('requester'),
    nonce: 1,
    workload: 'calcul/sha256',
    input: new Uint8Array(32).fill(1),
    inputSize: 100,
    width: 0,
    height: 0,
    fragments: 2,
    verificationsRequired: 1,
    leaseMs: 60_000,
    maxAttempts: 3,
    rewardPerUnit: 10,
    verifierRewardPerUnit: 1,
    expiresAtMs: 1_900_000_000_000,
  }
}

function eventFromVector(): EventInput {
  return {
    requester: hex('requester'),
    job: hex('job_id'),
    sequence: 1,
    previous: new Uint8Array(32),
    issuedAtMs: 1_800_000_000_000,
  }
}

const utf8 = (bytes: Uint8Array) => new TextDecoder().decode(bytes)

// Ed25519 public key as SubjectPublicKeyInfo (DER), so node:crypto can verify independently.
function ed25519Key(raw: Uint8Array) {
  const spki = Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), Buffer.from(raw)])
  return createPublicKey({ key: spki, format: 'der', type: 'spki' })
}

describe('the signed texts match sil-ledger byte for byte', () => {
  it('writes the spec text exactly as Rust does', () => {
    expect(toHex(specSigningMessage(specFromVector()))).toBe(vectors.spec_message)
    expect(utf8(specSigningMessage(specFromVector())).startsWith('Silicium JobSpec v2\n')).toBe(true)
  })

  it('writes the event text exactly as Rust does', () => {
    const body = encodeJobPublishedBody(specFromVector(), hex('spec_signature'))
    expect(toHex(eventSigningMessage(eventFromVector(), hex('spec_signature')))).toBe(vectors.event_message)
    expect(jobPublishedText(eventFromVector(), hex('spec_signature')).split('\n')[0]).toBe('Silicium Event v1')
    expect(encodeEventCore(eventFromVector(), body).length).toBeGreaterThan(0)
  })

  it('encodes the spec CBOR exactly as Rust does', () => {
    expect(toHex(encodeJobSpec(specFromVector()))).toBe(vectors.spec_cbor)
  })

  it('encodes the full signed event as the node stores it', () => {
    const body = encodeJobPublishedBody(specFromVector(), hex('spec_signature'))
    const event = encodeSignedEvent(eventFromVector(), body, hex('event_signature'))
    expect(toHex(event)).toBe(vectors.event_cbor)
  })

  it('derives the job id as SHA-256 of the job-id framed spec bytes', () => {
    const framed = domainFrame('sil/v1/job-id', encodeJobSpec(specFromVector()))
    expect(createHash('sha256').update(framed).digest('hex')).toBe(vectors.job_id)
  })

  it('the spec text names every field in fixed order', () => {
    const lines = specText(specFromVector()).split('\n')
    expect(lines.map((line) => line.split(':')[0])).toEqual([
      'Silicium JobSpec v2',
      'Requester',
      'Nonce',
      'Workload',
      'Input',
      'Input size',
      'Width',
      'Height',
      'Fragments',
      'Verifications',
      'Lease ms',
      'Max attempts',
      'Reward per unit',
      'Verifier reward per unit',
      'Expires ms',
    ])
  })
})

describe('the vector signatures are valid Ed25519 over the texts', () => {
  it('spec signature verifies with the requester key', () => {
    const key = ed25519Key(hex('requester'))
    const ok = verify(null, Buffer.from(hex('spec_message')), key, Buffer.from(hex('spec_signature')))
    expect(ok).toBe(true)
  })

  it('event signature verifies with the requester key', () => {
    const key = ed25519Key(hex('requester'))
    const ok = verify(null, Buffer.from(hex('event_message')), key, Buffer.from(hex('event_signature')))
    expect(ok).toBe(true)
  })
})
