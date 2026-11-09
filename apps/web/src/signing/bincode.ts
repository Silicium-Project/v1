// Encodage bincode 1 à entiers fixes et little-endian, comme `sil-chain-core::wire` (le format
// des instructions du programme d'escrow). Une suite d'octets (clé, identifiant, texte) est
// précédée de sa longueur en u64 ; un vecteur aussi (son nombre d'éléments en u64).

export class BincodeWriter {
  private readonly chunks: Uint8Array[] = []

  u32(value: number): this {
    const bytes = new Uint8Array(4)
    new DataView(bytes.buffer).setUint32(0, value, true)
    return this.raw(bytes)
  }

  u64(value: bigint | number): this {
    const bytes = new Uint8Array(8)
    new DataView(bytes.buffer).setBigUint64(0, BigInt(value), true)
    return this.raw(bytes)
  }

  /** Suite d'octets de longueur variable (une clé ou un identifiant de 32 octets en pratique). */
  bytes(value: Uint8Array): this {
    return this.u64(value.length).raw(value)
  }

  string(value: string): this {
    return this.bytes(new TextEncoder().encode(value))
  }

  raw(value: Uint8Array): this {
    this.chunks.push(value)
    return this
  }

  finish(): Uint8Array {
    const total = this.chunks.reduce((sum, chunk) => sum + chunk.length, 0)
    const out = new Uint8Array(total)
    let at = 0
    for (const chunk of this.chunks) {
      out.set(chunk, at)
      at += chunk.length
    }
    return out
  }
}
