use crate::{
    ContentError,
    merkle::{CHUNK_SIZE, chunk_count, content_id, leaf_hash, root},
};
use serde::{Deserialize, Serialize};
use sil_codec::{decode, encode};
use sil_types::{ContentId, Hash32};

/// Plus grande taille encodée acceptée pour un manifeste.
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;

/// Liste des hash de morceaux d'un contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub size: u64,
    pub leaves: Vec<Hash32>,
}

impl Manifest {
    /// Construit le manifeste d'un contenu en mémoire.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self {
            size: bytes.len() as u64,
            leaves: bytes.chunks(CHUNK_SIZE).map(leaf_hash).collect(),
        }
    }

    /// Identifiant que ce manifeste engage.
    pub fn id(&self) -> ContentId {
        content_id(self.size, &root(&self.leaves))
    }

    /// Vérifie la cohérence interne et la correspondance avec l'identifiant demandé.
    pub fn verify(&self, expected: &ContentId, max_object_bytes: u64) -> Result<(), ContentError> {
        if self.size > max_object_bytes {
            return Err(ContentError::TooLarge {
                size: self.size,
                max: max_object_bytes,
            });
        }
        if self.leaves.len() as u64 != chunk_count(self.size) {
            return Err(ContentError::BadManifest("nombre de morceaux"));
        }
        if self.id() != *expected {
            return Err(ContentError::WrongId);
        }
        Ok(())
    }

    /// Taille attendue du morceau `index` (`None` s'il n'existe pas).
    pub fn chunk_len(&self, index: u32) -> Option<usize> {
        let start = u64::from(index) * CHUNK_SIZE as u64;
        (start < self.size).then(|| (self.size - start).min(CHUNK_SIZE as u64) as usize)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ContentError> {
        encode(self).map_err(|_| ContentError::BadManifest("encodage"))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ContentError> {
        decode(bytes, MAX_MANIFEST_BYTES).map_err(|_| ContentError::BadManifest("décodage"))
    }
}
