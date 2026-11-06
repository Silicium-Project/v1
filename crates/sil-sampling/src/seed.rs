use sil_crypto::{Domain, SigningIdentity, sha256_domain, verify};
use sil_types::{Hash32, PublicKey, Signature, TaskRef};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SeedError {
    #[error("la signature de graine est invalide")]
    BadSignature,
}

/// Octets signés par le vérificateur : tâche, tentative, résultat jugé.
pub fn challenge_message(task: &TaskRef, attempt: u32, result: &Hash32) -> Vec<u8> {
    let mut message = Vec::with_capacity(72);
    message.extend_from_slice(task.job.as_bytes());
    message.extend_from_slice(&task.fragment.to_le_bytes());
    message.extend_from_slice(&attempt.to_le_bytes());
    message.extend_from_slice(result.as_bytes());
    message
}

/// Graine d'un tirage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplingSeed(Hash32);

impl SamplingSeed {
    fn from_signature(signature: &Signature) -> Self {
        Self(sha256_domain(Domain::Sampling, signature.as_bytes()))
    }

    /// Le vérificateur tire sa graine ; la signature doit accompagner son verdict.
    pub fn draw(
        identity: &SigningIdentity,
        task: &TaskRef,
        attempt: u32,
        result: &Hash32,
    ) -> (Self, Signature) {
        let message = challenge_message(task, attempt, result);
        let signature = identity.sign(Domain::Sampling, &message);
        (Self::from_signature(&signature), signature)
    }

    /// Revérifie une graine annoncée par un vérificateur.
    pub fn verify(
        verifier: &PublicKey,
        task: &TaskRef,
        attempt: u32,
        result: &Hash32,
        signature: &Signature,
    ) -> Result<Self, SeedError> {
        let message = challenge_message(task, attempt, result);
        verify(verifier, Domain::Sampling, &message, signature)
            .map_err(|_| SeedError::BadSignature)?;
        Ok(Self::from_signature(signature))
    }

    pub(crate) fn bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}
