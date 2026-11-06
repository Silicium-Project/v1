use crate::{AttestError, Attestation};
use serde::{Deserialize, Serialize};
use sil_types::{Hash32, PublicKey, TaskRef};
use std::collections::BTreeSet;

/// Plus grand nombre d'approbations dans un certificat.
pub const MAX_APPROVALS: usize = 16;

/// Preuve que ≥ k vérificateurs distincts ont approuvé un résultat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certificate {
    pub task: TaskRef,
    pub attempt: u32,
    pub result: Hash32,
    pub approvals: Vec<Attestation>,
}

/// Règles imposées par celui qui revalide le certificat.
pub struct CertRules<'a> {
    /// Nombre minimal d'approbations distinctes (≥ 1).
    pub required: u32,
    /// Clés interdites (l'auteur du résultat).
    pub excluded: &'a [PublicKey],
    /// Qui peut être vérificateur.
    pub eligible: &'a dyn Fn(&PublicKey) -> bool,
}

impl Certificate {
    /// Revalide entièrement le certificat, signatures comprises.
    pub fn verify(&self, rules: &CertRules<'_>) -> Result<(), AttestError> {
        if rules.required == 0 {
            return Err(AttestError::InvalidRequirement);
        }
        if self.approvals.len() > MAX_APPROVALS {
            return Err(AttestError::TooMany);
        }
        let mut seen = BTreeSet::new();
        for attestation in &self.approvals {
            if !seen.insert(attestation.verifier) {
                return Err(AttestError::DuplicateVerifier);
            }
            let body = &attestation.body;
            if body.task != self.task || body.attempt != self.attempt || body.result != self.result
            {
                return Err(AttestError::Mismatch("tâche, tentative ou résultat"));
            }
            if !body.approved {
                return Err(AttestError::NotApproval);
            }
            if rules.excluded.contains(&attestation.verifier) {
                return Err(AttestError::ExcludedVerifier);
            }
            if !(rules.eligible)(&attestation.verifier) {
                return Err(AttestError::IneligibleVerifier);
            }
            attestation.verify()?;
        }
        if seen.len() < usize::try_from(rules.required).unwrap_or(usize::MAX) {
            return Err(AttestError::TooFew {
                have: seen.len(),
                need: rules.required,
            });
        }
        Ok(())
    }
}
