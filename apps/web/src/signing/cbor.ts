// Minimal canonical CBOR (RFC 8949) for the shapes the ledger signs: unsigned integers,
// byte strings, text strings, arrays and maps with text keys. It must produce exactly the
// bytes of sil-codec (ciborium): shortest integer heads, definite lengths, struct fields in
// declaration order. contracts/vectors/job-published.json checks this.

const MAJOR_UINT = 0
const MAJOR_BYTES = 2
const MAJOR_TEXT = 3
const MAJOR_ARRAY = 4
const MAJOR_MAP = 5

function head(major: number, value: number | bigint): Uint8Array {
  const n = BigInt(value)
  if (n < 0n) throw new RangeError('negative CBOR head')
  const m = major << 5
  if (n < 24n) return Uint8Array.of(m | Number(n))
  if (n < 0x100n) return Uint8Array.of(m | 24, Number(n))
  if (n < 0x10000n) return Uint8Array.of(m | 25, Number(n >> 8n), Number(n & 0xffn))
  if (n < 0x100000000n) {
    const out = new Uint8Array(5)
    out[0] = m | 26
    new DataView(out.buffer).setUint32(1, Number(n))
    return out
  }
  if (n < 0x10000000000000000n) {
    const out = new Uint8Array(9)
    out[0] = m | 27
    new DataView(out.buffer).setBigUint64(1, n)
    return out
  }
  throw new RangeError('integer does not fit in 64 bits')
}

function concat(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, part) => sum + part.length, 0)
  const out = new Uint8Array(total)
  let offset = 0
  for (const part of parts) {
    out.set(part, offset)
    offset += part.length
  }
  return out
}

export const cbor = {
  uint(value: number | bigint): Uint8Array {
    return head(MAJOR_UINT, value)
  },
  bytes(value: Uint8Array): Uint8Array {
    return concat([head(MAJOR_BYTES, value.length), value])
  },
  text(value: string): Uint8Array {
    const encoded = new TextEncoder().encode(value)
    return concat([head(MAJOR_TEXT, encoded.length), encoded])
  },
  array(items: Uint8Array[]): Uint8Array {
    return concat([head(MAJOR_ARRAY, items.length), ...items])
  },
  /** Entries are written in the order given: the caller passes struct fields in declaration order. */
  map(entries: ReadonlyArray<readonly [string, Uint8Array]>): Uint8Array {
    const parts: Uint8Array[] = [head(MAJOR_MAP, entries.length)]
    for (const [key, value] of entries) {
      parts.push(cbor.text(key), value)
    }
    return concat(parts)
  },
}

/** `tag || 0x00 || message`: the bytes an Ed25519 signature covers (sil-crypto `Domain::frame`). */
export function domainFrame(tag: string, message: Uint8Array): Uint8Array {
  return concat([new TextEncoder().encode(tag), Uint8Array.of(0), message])
}

export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
}

export function fromHex(hex: string): Uint8Array {
  if (hex.length % 2 !== 0 || !/^[0-9a-f]*$/.test(hex)) throw new Error('invalid hex')
  const out = new Uint8Array(hex.length / 2)
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(2 * i, 2 * i + 2), 16)
  return out
}
