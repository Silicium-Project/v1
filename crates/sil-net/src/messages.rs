//! Messages de requête-réponse.

use crate::wire::Bytes;
use serde::{Deserialize, Serialize};
use sil_types::{ContentId, JobId, PublicKey};

/// Demande d'événements d'un job à un pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerRequest {
    /// Les événements de `author` après `after_sequence` (combler un trou précis).
    Author {
        job: JobId,
        author: PublicKey,
        after_sequence: u64,
        max: u16,
    },
    /// Les événements du job à partir de la position `cursor` du journal **du pair**
    /// (synchronisation d'un nœud retardataire : il rappelle avec `cursor += reçus`).
    Cursor { job: JobId, cursor: u64, max: u16 },
}

/// Événements encodés (`sil-ledger::Event::to_bytes`), en ordre de séquence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerResponse {
    Events(Vec<Bytes>),
    /// Limite de débit dépassée : réessayer plus tard ou demander à un autre pair.
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentRequest {
    Manifest { id: ContentId },
    Chunk { id: ContentId, index: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentResponse {
    Manifest(Bytes),
    Chunk(Bytes),
    NotFound,
    Busy,
}

impl LedgerResponse {
    /// Événements encodés, si la réponse en contient.
    pub fn into_events(self) -> Option<Vec<Vec<u8>>> {
        match self {
            Self::Events(events) => Some(events.into_iter().map(|b| b.0).collect()),
            Self::Busy => None,
        }
    }
}
