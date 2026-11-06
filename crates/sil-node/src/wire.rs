//! Enveloppe des messages gossip des topics `announce` et `jobs/*`.

use serde::{Deserialize, Serialize};
use sil_codec::{CodecError, decode, encode};
use sil_ledger::Event;
use sil_settlement::SettlementClaim;
use sil_types::{PublicKey, Signature};

/// Taille maximale d'une enveloppe (un événement fait au plus 32 Kio).
pub const MAX_ENVELOPE_BYTES: usize = 40 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Envelope {
    /// Événement du journal. Boxé : une spec de job est volumineuse, et l'écart de taille entre
    /// variantes ferait échouer clippy (`large_enum_variant`). Le format sur le fil ne change pas.
    Event(Box<Event>),
    /// Signature d'un membre du comité sur un règlement.
    SettlementSig {
        claim: Box<SettlementClaim>,
        signer: PublicKey,
        signature: Signature,
    },
}

impl Envelope {
    pub fn to_bytes(&self) -> Result<Vec<u8>, CodecError> {
        encode(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CodecError> {
        decode(bytes, MAX_ENVELOPE_BYTES)
    }
}
