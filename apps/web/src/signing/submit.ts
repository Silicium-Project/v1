import { domainFrame, fromHex, toHex } from './cbor'
import {
  encodeJobPublishedBody,
  encodeJobSpec,
  encodeSignedEvent,
  eventSigningMessage,
  specSigningMessage,
  type EventInput,
  type JobSpecInput,
} from './job'

// Turns a user's request into the three things the node needs: the input content id, a spec
// signed by the requester's wallet, and a JobPublished event signed by the same wallet.
// Everything external (upload, wallet, publication, hashing, clock) is injected, so the
// logic is tested without a browser.

export interface SubmitDeps {
  putContent(data: Uint8Array): Promise<{ content_id: string }>
  /** `display` tells the wallet how to show the message. Both signed texts are UTF-8. */
  sign(message: Uint8Array, display: 'utf8' | 'hex'): Promise<Uint8Array>
  publish(jobId: string, event: Uint8Array, label: string): Promise<{ id: string }>
  sha256(data: Uint8Array): Promise<Uint8Array>
  now(): number
  randomNonce(): number
  /** Called before each step, so the UI can show where the submission is. */
  progress?(step: string): void
}

export interface SubmitInput {
  requester: Uint8Array // the wallet's 32-byte public key
  data: Uint8Array // the input file (for rendu/blocs: the scene, without the size header)
  workload: string
  fragments: number
  committee: Uint8Array[]
  threshold: number
  label: string
  /** Image size in pixels, used by rendu/blocs only. */
  width?: number
  height?: number
}

export const ONE_DAY_MS = 24 * 60 * 60 * 1000
export const LEASE_MS = 60_000
export const MAX_ATTEMPTS = 3
export const RENDER_WORKLOAD = 'rendu/blocs'
/** Largest side of an image, in pixels (the render workload refuses more). */
export const MAX_SIDE = 16_384

/** True for a whole number of pixels from 1 to MAX_SIDE. */
export function isSide(value: number): boolean {
  return Number.isInteger(value) && value >= 1 && value <= MAX_SIDE
}

/**
 * The bytes the node stores as the job input. For rendu/blocs the size goes in front of the scene:
 * width then height, each u32 little-endian (sil-wl-render, `Header`).
 */
export function inputPayload(input: SubmitInput): { payload: Uint8Array; width: number; height: number } {
  if (input.workload !== RENDER_WORKLOAD) {
    return { payload: input.data, width: 0, height: 0 }
  }
  const width = input.width ?? 0
  const height = input.height ?? 0
  if (!isSide(width) || !isSide(height)) {
    throw new Error('image size must be whole numbers from 1 to 16384')
  }
  const payload = new Uint8Array(8 + input.data.length)
  const view = new DataView(payload.buffer)
  view.setUint32(0, width, true)
  view.setUint32(4, height, true)
  payload.set(input.data, 8)
  return { payload, width, height }
}

export interface SubmitResult {
  jobId: string
  steps: string[] // the steps that ran, for the UI log
}

/** Runs the four steps and returns the job id (hex) the node accepted. */
export async function submitJob(input: SubmitInput, deps: SubmitDeps): Promise<SubmitResult> {
  const steps: string[] = []
  const advance = (step: string) => {
    steps.push(step)
    deps.progress?.(step)
  }
  const { payload, width, height } = inputPayload(input)

  advance('upload')
  const receipt = await deps.putContent(payload)
  const contentId = fromHex(receipt.content_id)

  const spec: JobSpecInput = {
    requester: input.requester,
    nonce: deps.randomNonce(),
    workload: input.workload,
    input: contentId,
    inputSize: payload.length,
    width,
    height,
    fragments: input.fragments,
    committee: input.committee,
    threshold: input.threshold,
    verificationsRequired: 1,
    leaseMs: LEASE_MS,
    maxAttempts: MAX_ATTEMPTS,
    rewardPerUnit: 0, // the site does not set a payment yet
    verifierRewardPerUnit: 0,
    expiresAtMs: deps.now() + ONE_DAY_MS,
  }

  advance('sign-spec')
  const specSignature = await deps.sign(specSigningMessage(spec), 'utf8')

  // job_id = SHA-256 over the job-id framed spec bytes (sil-ledger JobSpec::job_id).
  const jobId = toHex(await deps.sha256(domainFrame('sil/v1/job-id', encodeJobSpec(spec))))

  const event: EventInput = {
    requester: input.requester,
    job: fromHex(jobId),
    sequence: 1,
    previous: new Uint8Array(32),
    issuedAtMs: deps.now(),
  }
  const body = encodeJobPublishedBody(spec, specSignature)

  advance('sign-event')
  const eventSignature = await deps.sign(eventSigningMessage(event, specSignature), 'utf8')

  advance('publish')
  const published = await deps.publish(jobId, encodeSignedEvent(event, body, eventSignature), input.label)
  if (published.id !== jobId) {
    throw new Error('the node answered with another job id')
  }
  return { jobId, steps }
}

