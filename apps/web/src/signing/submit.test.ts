// @vitest-environment node
import { createHash, createPublicKey, generateKeyPairSync, sign, verify } from 'node:crypto'
import { describe, expect, it } from 'vitest'
import { fromHex, toHex } from './cbor'
import { submitJob, type SubmitDeps } from './submit'
import { domainFrame } from './cbor'
import { encodeJobSpec, type JobSpecInput } from './job'
import { ONE_DAY_MS } from './submit'

// A real Ed25519 key plays the wallet. Signatures are checked with node:crypto afterwards, so
// the test proves the submission pipeline produces valid requester signatures, not just bytes.
const { privateKey, publicKey } = generateKeyPairSync('ed25519')
const rawPublic = new Uint8Array(publicKey.export({ format: 'der', type: 'spki' }).subarray(-32))
const committee = [1, 2, 3].map((n) => new Uint8Array(32).fill(n))

interface Recorded {
  messages: Array<{ bytes: Uint8Array; display: string; signature: Buffer }>
  published?: { jobId: string; event: Uint8Array }
}

function fakes(recorded: Recorded) {
  let jobId = ''
  const deps: SubmitDeps = {
    async putContent(data) {
      return { content_id: toHex(createHash('sha256').update(data).digest()) }
    },
    async sign(message, display) {
      const signature = sign(null, Buffer.from(message), privateKey)
      recorded.messages.push({ bytes: message, display, signature })
      return new Uint8Array(signature)
    },
    async publish(id, event, label) {
      jobId = id
      recorded.published = { jobId: id, event }
      expect(label).toBe('mon calcul')
      return { id }
    },
    async sha256(data) {
      return new Uint8Array(createHash('sha256').update(data).digest())
    },
    now: () => 1_800_000_000_000,
    randomNonce: () => 42,
  }
  return { deps, jobIdOf: () => jobId }
}

const input = {
  requester: rawPublic,
  data: new TextEncoder().encode('hello'),
  workload: 'calcul/sha256',
  fragments: 2,
  committee,
  threshold: 2,
  label: 'mon calcul',
}

describe('submitJob', () => {
  it('runs the four steps in order and returns the job id the node accepted', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    const result = await submitJob(input, deps)
    expect(result.steps).toEqual(['upload', 'sign-spec', 'sign-event', 'publish'])
    expect(result.jobId).toMatch(/^[0-9a-f]{64}$/)
    expect(recorded.published?.jobId).toBe(result.jobId)
  })

  it('asks the wallet to sign exactly two framed messages, binary shown as hex', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    await submitJob(input, deps)
    expect(recorded.messages).toHaveLength(2)
    expect(recorded.messages.map((m) => m.display)).toEqual(['utf8', 'utf8'])
    expect(new TextDecoder().decode(recorded.messages[0].bytes).startsWith('Silicium JobSpec v1\n')).toBe(true)
    expect(new TextDecoder().decode(recorded.messages[1].bytes).startsWith('Silicium Event v1\n')).toBe(true)
  })

  it('produces signatures that the requester key verifies over the framed messages', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    await submitJob(input, deps)
    const key = createPublicKey({ key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), Buffer.from(rawPublic)]), format: 'der', type: 'spki' })
    for (const message of recorded.messages) {
      expect(verify(null, Buffer.from(message.bytes), key, message.signature)).toBe(true)
    }
  })

  it('gives the job id as SHA-256 of the job-id framed spec CBOR, which the node also derives', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    const result = await submitJob(input, deps)
    const spec: JobSpecInput = {
      requester: rawPublic,
      nonce: 42,
      workload: 'calcul/sha256',
      input: new Uint8Array(createHash('sha256').update(input.data).digest()),
      inputSize: input.data.length,
      width: 0,
      height: 0,
      fragments: 2,
      committee,
      threshold: 2,
      verificationsRequired: 1,
      leaseMs: 60_000,
      maxAttempts: 3,
      rewardPerUnit: 0,
      verifierRewardPerUnit: 0,
      expiresAtMs: 1_800_000_000_000 + ONE_DAY_MS,
    }
    const expected = createHash('sha256').update(domainFrame('sil/v1/job-id', encodeJobSpec(spec))).digest('hex')
    expect(result.jobId).toBe(expected)
  })

  it('puts the image size in front of a render scene and signs it', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    let stored: Uint8Array = new Uint8Array()
    deps.putContent = async (data) => {
      stored = data
      return { content_id: toHex(createHash('sha256').update(data).digest()) }
    }
    const scene = new TextEncoder().encode('scene')
    await submitJob({ ...input, workload: 'rendu/blocs', data: scene, width: 640, height: 480 }, deps)
    // 640 et 480 en u32 little-endian, puis la scène telle quelle.
    expect(Array.from(stored.subarray(0, 8))).toEqual([128, 2, 0, 0, 224, 1, 0, 0])
    expect(Array.from(stored.subarray(8))).toEqual(Array.from(scene))
    const specText = new TextDecoder().decode(recorded.messages[0].bytes)
    expect(specText).toContain('\nWidth: 640\nHeight: 480\n')
    expect(specText).toContain(`\nInput size: ${stored.length}\n`)
  })

  it('refuses a render job whose image size is not a whole number of pixels', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    const scene = new TextEncoder().encode('scene')
    await expect(
      submitJob({ ...input, workload: 'rendu/blocs', data: scene, width: 0, height: 480 }, deps),
    ).rejects.toThrow('image size')
    expect(recorded.messages).toHaveLength(0)
  })

  it('refuses when the node answers with another job id', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    deps.publish = async () => ({ id: 'ff'.repeat(32) })
    await expect(submitJob(input, deps)).rejects.toThrow('another job id')
  })

  it('stops before signing when the upload fails', async () => {
    const recorded: Recorded = { messages: [] }
    const { deps } = fakes(recorded)
    deps.putContent = async () => {
      throw new Error('network')
    }
    await expect(submitJob(input, deps)).rejects.toThrow('network')
    expect(recorded.messages).toHaveLength(0)
    expect(fromHex('00'.repeat(32))).toHaveLength(32)
  })
})
