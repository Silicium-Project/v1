// Base58 (Bitcoin/Solana alphabet): the text form of a 32-byte wallet address.

const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'

export function encodeBase58(bytes: Uint8Array): string {
  let zeros = 0
  while (zeros < bytes.length && bytes[zeros] === 0) zeros++
  // Big-number conversion by repeated division of a byte array (addresses are 32 bytes).
  const digits: number[] = []
  for (const byte of bytes) {
    let carry = byte
    for (let i = 0; i < digits.length; i++) {
      carry += digits[i] << 8
      digits[i] = carry % 58
      carry = Math.floor(carry / 58)
    }
    while (carry > 0) {
      digits.push(carry % 58)
      carry = Math.floor(carry / 58)
    }
  }
  let text = '1'.repeat(zeros)
  for (let i = digits.length - 1; i >= 0; i--) text += ALPHABET[digits[i]]
  return text
}

export function decodeBase58(text: string): Uint8Array {
  if (text.length === 0) throw new Error('invalid base58')
  const bytes: number[] = []
  for (const char of text) {
    const value = ALPHABET.indexOf(char)
    if (value < 0) throw new Error('invalid base58')
    let carry = value
    for (let i = 0; i < bytes.length; i++) {
      carry += bytes[i] * 58
      bytes[i] = carry & 0xff
      carry >>= 8
    }
    while (carry > 0) {
      bytes.push(carry & 0xff)
      carry >>= 8
    }
  }
  let zeros = 0
  while (zeros < text.length && text[zeros] === '1') zeros++
  const out = new Uint8Array(zeros + bytes.length)
  for (let i = 0; i < bytes.length; i++) out[out.length - 1 - i] = bytes[i]
  return out
}
