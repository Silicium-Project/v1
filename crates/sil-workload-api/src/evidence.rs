use serde::{Deserialize, Serialize};
use sil_codec::encode;
use sil_crypto::sha256;
use sil_types::Hash32;

/// Preuve de divergence : le bloc, ce qui était engagé, ce que le vérificateur a obtenu.
/// Son hash est cité dans l'attestation de rejet ; le contenu est stocké pour examen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub block: u32,
    pub committed: Hash32,
    pub recomputed: Hash32,
    /// Raison courte (au plus 100 caractères).
    pub reason: String,
}

impl Evidence {
    pub fn new(block: u32, committed: Hash32, recomputed: Hash32, reason: &str) -> Self {
        Self {
            block,
            committed,
            recomputed,
            reason: reason.chars().take(100).collect(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        encode(self).unwrap_or_default()
    }

    /// Hash à mettre dans `AttestationBody::evidence`.
    pub fn hash(&self) -> Hash32 {
        sha256(&self.to_bytes())
    }
}

/// Conclusion d'une vérification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Approve,
    /// Un rejet est toujours accompagné de sa preuve.
    Reject(Evidence),
}
