import { cbor, fromHex, toHex } from './cbor'
import type { EventInput } from './job'

// Annulation d'un calcul par son requester (sil-ledger `EventBody::Cancel`). Comme JobPublished, le
// texte signé est en ASCII lisible : le wallet l'affiche tel quel. Le texte doit être identique,
// octet pour octet, à `cancel_text` dans sil-ledger src/wallet_text.rs.

/** Tête de la chaîne du requester, telle que le nœud la donne sur GET /v1/jobs/{id}. */
export interface RequesterHead {
  sequence: number
  hash: string // hex, identifiant du dernier événement
}

/** The text the requester signs to cancel its job. */
export function cancelText(event: EventInput): string {
  return [
    'Silicium Event v1',
    `Author: ${toHex(event.requester)}`,
    `Job: ${toHex(event.job)}`,
    `Sequence: ${event.sequence}`,
    `Previous: ${toHex(event.previous)}`,
    `Issued ms: ${event.issuedAtMs}`,
    'Body: Cancel',
  ].join('\n')
}

/** Bytes the wallet signs for the cancellation (UTF-8 of cancelText). */
export function cancelSigningMessage(event: EventInput): Uint8Array {
  return new TextEncoder().encode(cancelText(event))
}

/** Next event of the requester chain: one after its head, which it follows. */
export function cancelEvent(job: Uint8Array, requester: Uint8Array, head: RequesterHead, issuedAtMs: number): EventInput {
  return {
    requester,
    job,
    sequence: head.sequence + 1,
    previous: fromHex(head.hash),
    issuedAtMs,
  }
}

/** EventBody::Cancel is a unit variant: its CBOR is the text "Cancel". */
export function encodeCancelBody(): Uint8Array {
  return cbor.text('Cancel')
}
