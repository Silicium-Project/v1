import { describe, expect, it } from 'vitest'
import { encodeSignedEvent } from './job'
import { cancelEvent, cancelText, encodeCancelBody } from './cancel'
import { toHex } from './cbor'

const requester = new Uint8Array(32).fill(7)
const job = new Uint8Array(32).fill(9)
const head = { sequence: 1, hash: 'ab'.repeat(32) }

describe('cancellation event', () => {
  it('is the next event of the requester chain', () => {
    const event = cancelEvent(job, requester, head, 5_000)
    expect(event.sequence).toBe(2)
    expect(toHex(event.previous)).toBe('ab'.repeat(32))
  })

  it('signs the same text as sil-ledger wallet_text::cancel_text', () => {
    const event = cancelEvent(job, requester, head, 5_000)
    expect(cancelText(event)).toBe(
      [
        'Silicium Event v1',
        `Author: ${'07'.repeat(32)}`,
        `Job: ${'09'.repeat(32)}`,
        'Sequence: 2',
        `Previous: ${'ab'.repeat(32)}`,
        'Issued ms: 5000',
        'Body: Cancel',
      ].join('\n'),
    )
  })

  it('encodes the body as the CBOR text "Cancel" (parity with the Rust test)', () => {
    expect(toHex(encodeCancelBody())).toBe('66' + Buffer.from('Cancel').toString('hex'))
  })

  it('wraps the body into a signed event', () => {
    const event = cancelEvent(job, requester, head, 5_000)
    const signed = encodeSignedEvent(event, encodeCancelBody(), new Uint8Array(64))
    expect(signed.length).toBeGreaterThan(64)
  })
})
