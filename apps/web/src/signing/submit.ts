import { findWorkload, isImageSide } from '@/workloads/find'
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
  /** Opens the job's escrow on-chain, approved by the wallet, before the job is published. */
  deposit(spec: JobSpecInput, jobId: string): Promise<void>
  sha256(data: Uint8Array): Promise<Uint8Array>
  now(): number
  randomNonce(): number
  /** Called before each step, so the UI can show where the submission is. */
  progress?(step: string): void
}

/** The job's terms, set by the network (services/api/internal/policy); the user never chooses them. */
export interface JobTerms {
  fragments: number
  /** Paid to the worker, per unit of its fragment (lamports). */
  rewardPerUnit: number
  /** Paid to each verifier, per unit of the fragment (lamports). */
  verifierRewardPerUnit: number
  verificationsRequired: number
  leaseMs: number
  maxAttempts: number
  expiresInMs: number
}

export interface SubmitInput {
  requester: Uint8Array // the wallet's 32-byte public key
  data: Uint8Array // the input file, without the parameters (they go in front of it)
  workload: string
  terms: JobTerms
  label: string
  /** The workload's declared parameters (`params` of its descriptor), e.g. width and height. */
  params?: Partial<Record<'width' | 'height', number>>
}

/**
 * The bytes the node stores as the job input. The parameters the workload declares go in front of
 * the file, each as a u32 little-endian, in the order of its descriptor (sil-wl-render, `Header`).
 * A workload without parameters stores the file as it is.
 */
export function inputPayload(input: SubmitInput): { payload: Uint8Array; width: number; height: number } {
  const spec = findWorkload(input.workload)
  if (!spec) {
    throw new Error(`unknown workload: ${input.workload}`)
  }
  const values = spec.params.map((name) => input.params?.[name] ?? Number.NaN)
  if (!values.every((value) => isImageSide(value, spec))) {
    throw new Error(`parameters must be whole numbers from 1 to ${spec.maxSide}`)
  }
  const dimension = (name: 'width' | 'height') => {
    const index = spec.params.indexOf(name)
    return index === -1 ? 0 : values[index]!
  }
  if (values.length === 0) {
    return { payload: input.data, width: 0, height: 0 }
  }
  const header = 4 * values.length
  const payload = new Uint8Array(header + input.data.length)
  const view = new DataView(payload.buffer)
  values.forEach((value, index) => view.setUint32(4 * index, value, true))
  payload.set(input.data, header)
  return { payload, width: dimension('width'), height: dimension('height') }
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
    fragments: input.terms.fragments,
    verificationsRequired: input.terms.verificationsRequired,
    leaseMs: input.terms.leaseMs,
    maxAttempts: input.terms.maxAttempts,
    rewardPerUnit: input.terms.rewardPerUnit,
    verifierRewardPerUnit: input.terms.verifierRewardPerUnit,
    expiresAtMs: deps.now() + input.terms.expiresInMs,
  }

  advance('sign-spec')
  const specSignature = await deps.sign(specSigningMessage(spec), 'utf8')

  // job_id = SHA-256 over the job-id framed spec bytes (sil-ledger JobSpec::job_id).
  const jobId = toHex(await deps.sha256(domainFrame('sil/v1/job-id', encodeJobSpec(spec))))

  // L'escrow doit exister avant la publication : sans lui, aucun worker ne réclame le job.
  advance('deposit')
  await deps.deposit(spec, jobId)

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

