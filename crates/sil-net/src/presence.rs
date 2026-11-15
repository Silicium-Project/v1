//! Présence minimale : remplace le heartbeat JSON complet de V0 (toutes les 5 s, avec
//! télémétrie, activité et capacité). Ici quelques octets, envoyés seulement si l'état change
//! (au plus tous les `min_gap_ms`) ou après `max_silence_ms` de silence.

use serde::{Deserialize, Serialize};
use sil_codec::{decode, encode};
use sil_crypto::sha256;
use sil_types::Hash32;

pub const ROLE_COMPUTE: u8 = 1;
pub const ROLE_VERIFY: u8 = 2;
pub const ROLE_STORAGE: u8 = 4;
/// Machine qui peut réclamer des fragments : elle lit la chaîne (ancrage des claims).
pub const ROLE_CLAIM: u8 = 8;

const MAX_WORKLOADS: usize = 8;
const MAX_WORKLOAD_LEN: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Presence {
    pub version: u8,
    /// Masque de `ROLE_*`.
    pub roles: u8,
    /// Charge de 0 à 100.
    pub load: u8,
    /// Workloads acceptés (au plus 8).
    pub workloads: Vec<String>,
}

impl Presence {
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        encode(self).ok()
    }

    /// Décode et valide ; `None` si le message est mal formé (donc à rejeter).
    pub fn from_bytes(bytes: &[u8], max_len: usize) -> Option<Self> {
        let presence: Self = decode(bytes, max_len).ok()?;
        let valid = presence.version == 1
            && presence.load <= 100
            && presence.workloads.len() <= MAX_WORKLOADS
            && presence
                .workloads
                .iter()
                .all(|w| !w.is_empty() && w.len() <= MAX_WORKLOAD_LEN);
        valid.then_some(presence)
    }

    fn fingerprint(&self) -> Option<Hash32> {
        Some(sha256(&self.to_bytes()?))
    }
}

/// Décide quand republier la présence.
#[derive(Debug, Clone)]
pub struct PresenceGate {
    last: Option<(Hash32, u64)>,
    pub min_gap_ms: u64,
    pub max_silence_ms: u64,
}

impl PresenceGate {
    pub fn new(min_gap_ms: u64, max_silence_ms: u64) -> Self {
        Self {
            last: None,
            min_gap_ms,
            max_silence_ms,
        }
    }

    /// Vrai (et mémorise) s'il faut publier `presence` à `now_ms`.
    pub fn should_send(&mut self, presence: &Presence, now_ms: u64) -> bool {
        let Some(fingerprint) = presence.fingerprint() else {
            return false;
        };
        let send = match self.last {
            None => true,
            Some((last, at)) => {
                let elapsed = now_ms.saturating_sub(at);
                (last != fingerprint && elapsed >= self.min_gap_ms)
                    || elapsed >= self.max_silence_ms
            }
        };
        if send {
            self.last = Some((fingerprint, now_ms));
        }
        send
    }
}
