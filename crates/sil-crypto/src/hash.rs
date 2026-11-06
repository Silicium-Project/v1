use crate::Domain;
use sha2::{Digest, Sha256};
use sil_types::Hash32;

/// SHA-256 d'un message.
pub fn sha256(data: &[u8]) -> Hash32 {
    Hash32::from_bytes(Sha256::digest(data).into())
}

/// SHA-256 d'un message dans un domaine (ex. `job_id = sha256_domain(JobSpec, spec_bytes)`).
pub fn sha256_domain(domain: Domain, data: &[u8]) -> Hash32 {
    sha256(&domain.frame(data))
}
