//! Attestation et certificat en mode tolérance (embedding, docs/technical/embedding-workload.md, §10).
//!
//! Un vérificateur tolérance ne prouve pas des octets identiques : il prouve que chaque élément
//! tiré a un cosinus ≥ `cos_min` avec son propre recalcul, avec le même modèle et le même runtime.
//! Le message signé ne contient aucun flottant : le cosinus minimal est en millionièmes.
//! Domaine de signature distinct (`AttestationTolerance`) : une attestation exacte ne se rejoue pas ici.

use crate::AttestError;
use serde::{Deserialize, Serialize};
use sil_codec::encode;
use sil_crypto::{Domain, SigningIdentity, verify};
use sil_types::{Hash32, PublicKey, Signature, TaskRef};

/// Contenu signé d'un verdict tolérance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToleranceBody {
    pub task: TaskRef,
    pub attempt: u32,
    /// Hash du résultat du worker : c'est ce qui est jugé.
    pub result: Hash32,
    /// SHA-256 du fichier modèle sur lequel le vérificateur a recalculé.
    pub model: Hash32,
    /// Hash de la version du runtime (ONNX Runtime) utilisée pour le recalcul.
    pub runtime: Hash32,
    /// Hash de la liste des éléments tirés et des vecteurs recalculés.
    pub sample_root: Hash32,
    /// Plus petit cosinus observé sur les éléments tirés, en millionièmes.
    pub min_cos_micro: i32,
    pub approved: bool,
    /// Preuve de divergence : obligatoire pour un rejet, interdite pour une approbation.
    pub evidence: Option<Hash32>,
}

impl ToleranceBody {
    fn check_shape(&self) -> Result<(), AttestError> {
        match (self.approved, self.evidence.is_some()) {
            (true, true) => Err(AttestError::UnexpectedEvidence),
            (false, false) => Err(AttestError::MissingEvidence),
            _ => Ok(()),
        }
    }

    fn bytes(&self) -> Result<Vec<u8>, AttestError> {
        encode(self).map_err(|_| AttestError::Encode)
    }
}

/// Verdict signé d'un vérificateur tolérance, avec sa signature de règlement si approbation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToleranceAttestation {
    pub body: ToleranceBody,
    pub verifier: PublicKey,
    pub signature: Signature,
    pub settlement: Option<Signature>,
}

impl ToleranceAttestation {
    /// Signe un verdict tolérance.
    pub fn sign(identity: &SigningIdentity, body: ToleranceBody) -> Result<Self, AttestError> {
        body.check_shape()?;
        let signature = identity.sign(Domain::AttestationTolerance, &body.bytes()?);
        Ok(Self {
            body,
            verifier: identity.public(),
            signature,
            settlement: None,
        })
    }

    /// Ajoute la signature de règlement d'une approbation (même message que le mode exact).
    pub fn with_settlement(mut self, identity: &SigningIdentity, message: &[u8]) -> Self {
        if self.body.approved {
            self.settlement = Some(identity.sign(Domain::Settlement, message));
        }
        self
    }

    /// Vérifie la forme et la signature du verdict, sous le domaine tolérance.
    pub fn verify(&self) -> Result<(), AttestError> {
        self.body.check_shape()?;
        verify(
            &self.verifier,
            Domain::AttestationTolerance,
            &self.body.bytes()?,
            &self.signature,
        )
        .map_err(|_| AttestError::BadSignature)
    }
}

/// Preuve qu'au moins `required` vérificateurs tolérance ont approuvé un résultat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToleranceCertificate {
    pub task: TaskRef,
    pub attempt: u32,
    pub result: Hash32,
    pub approvals: Vec<ToleranceAttestation>,
}

/// Règles imposées par celui qui revalide : modèle, runtime, seuil et qui peut voter.
pub struct ToleranceRules<'a> {
    pub required: u32,
    pub excluded: &'a [PublicKey],
    pub eligible: &'a dyn Fn(&PublicKey) -> bool,
    /// Modèle attendu, pris du descripteur.
    pub model: Hash32,
    /// Runtime attendu, pris du descripteur.
    pub runtime: Hash32,
    /// Seuil de cosinus, en millionièmes, pris du descripteur.
    pub cos_min_micro: i32,
}

impl ToleranceCertificate {
    /// Revalide le certificat sans faire confiance à celui qui le présente.
    pub fn verify(&self, rules: &ToleranceRules<'_>) -> Result<(), AttestError> {
        if rules.required == 0 {
            return Err(AttestError::InvalidRequirement);
        }
        if self.approvals.len() > crate::MAX_APPROVALS {
            return Err(AttestError::TooMany);
        }
        let mut seen = std::collections::BTreeSet::new();
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
            if body.model != rules.model || body.runtime != rules.runtime {
                return Err(AttestError::Mismatch("modèle ou runtime"));
            }
            if body.min_cos_micro < rules.cos_min_micro {
                return Err(AttestError::Mismatch("cosinus sous le seuil"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attestation, AttestationBody};
    use sil_crypto::{MasterSeed, Purpose};
    use sil_types::JobId;

    const COS_MIN: i32 = 999_000;

    fn id(n: u8) -> SigningIdentity {
        MasterSeed::from_bytes([n; 32]).derive(Purpose::Ledger)
    }

    fn task() -> TaskRef {
        TaskRef::new(JobId::from_bytes([1; 32]), 0)
    }

    fn hash(byte: u8) -> Hash32 {
        Hash32::from_bytes([byte; 32])
    }

    fn model() -> Hash32 {
        hash(0xAA)
    }

    fn runtime() -> Hash32 {
        hash(0xBB)
    }

    fn body(min_cos_micro: i32, approved: bool) -> ToleranceBody {
        ToleranceBody {
            task: task(),
            attempt: 1,
            result: hash(7),
            model: model(),
            runtime: runtime(),
            sample_root: hash(9),
            min_cos_micro,
            approved,
            evidence: if approved { None } else { Some(hash(3)) },
        }
    }

    fn approve(who: &SigningIdentity, min_cos_micro: i32) -> ToleranceAttestation {
        ToleranceAttestation::sign(who, body(min_cos_micro, true)).unwrap()
    }

    fn cert(approvals: Vec<ToleranceAttestation>) -> ToleranceCertificate {
        ToleranceCertificate {
            task: task(),
            attempt: 1,
            result: hash(7),
            approvals,
        }
    }

    fn anyone(_: &PublicKey) -> bool {
        true
    }

    fn rules<'a>(required: u32, excluded: &'a [PublicKey]) -> ToleranceRules<'a> {
        ToleranceRules {
            required,
            excluded,
            eligible: &anyone,
            model: model(),
            runtime: runtime(),
            cos_min_micro: COS_MIN,
        }
    }

    #[test]
    fn a_signed_approval_verifies() {
        assert!(approve(&id(1), 999_500).verify().is_ok());
    }

    #[test]
    fn an_exact_attestation_cannot_be_replayed_as_tolerance() {
        let exact = Attestation::sign(
            &id(1),
            AttestationBody {
                task: task(),
                attempt: 1,
                result: hash(7),
                approved: true,
                evidence: None,
            },
        )
        .unwrap();
        let replayed = ToleranceAttestation {
            body: body(999_500, true),
            verifier: exact.verifier,
            signature: exact.signature,
            settlement: None,
        };
        assert_eq!(replayed.verify(), Err(AttestError::BadSignature));
    }

    #[test]
    fn a_quorum_of_honest_approvals_passes() {
        let a = id(1);
        let b = id(2);
        let c = cert(vec![approve(&a, 999_500), approve(&b, 999_900)]);
        assert!(c.verify(&rules(2, &[])).is_ok());
    }

    #[test]
    fn an_approval_under_the_threshold_is_refused() {
        let c = cert(vec![approve(&id(1), 998_000)]);
        assert_eq!(
            c.verify(&rules(1, &[])),
            Err(AttestError::Mismatch("cosinus sous le seuil"))
        );
    }

    #[test]
    fn another_model_or_runtime_is_refused() {
        let c = cert(vec![approve(&id(1), 999_500)]);
        let mut other = rules(1, &[]);
        other.model = hash(0xCC);
        assert_eq!(
            c.verify(&other),
            Err(AttestError::Mismatch("modèle ou runtime"))
        );
        let mut other_runtime = rules(1, &[]);
        other_runtime.runtime = hash(0xDD);
        assert_eq!(
            c.verify(&other_runtime),
            Err(AttestError::Mismatch("modèle ou runtime"))
        );
    }

    #[test]
    fn a_rejection_is_not_an_approval_in_a_certificate() {
        let rejection = ToleranceAttestation::sign(&id(1), body(990_000, false)).unwrap();
        let c = cert(vec![rejection]);
        assert_eq!(c.verify(&rules(1, &[])), Err(AttestError::NotApproval));
    }

    #[test]
    fn evidence_follows_the_verdict() {
        let mut bad = body(999_500, true);
        bad.evidence = Some(hash(3));
        assert_eq!(
            ToleranceAttestation::sign(&id(1), bad).err(),
            Some(AttestError::UnexpectedEvidence)
        );
        let mut missing = body(990_000, false);
        missing.evidence = None;
        assert_eq!(
            ToleranceAttestation::sign(&id(1), missing).err(),
            Some(AttestError::MissingEvidence)
        );
    }

    #[test]
    fn duplicates_and_too_few_are_refused() {
        let a = id(1);
        let dup = cert(vec![approve(&a, 999_500), approve(&a, 999_500)]);
        assert_eq!(
            dup.verify(&rules(2, &[])),
            Err(AttestError::DuplicateVerifier)
        );
        let one = cert(vec![approve(&a, 999_500)]);
        assert_eq!(
            one.verify(&rules(2, &[])),
            Err(AttestError::TooFew { have: 1, need: 2 })
        );
    }

    #[test]
    fn the_result_must_match_the_certificate() {
        let mut other = body(999_500, true);
        other.result = hash(8);
        let c = cert(vec![ToleranceAttestation::sign(&id(1), other).unwrap()]);
        assert_eq!(
            c.verify(&rules(1, &[])),
            Err(AttestError::Mismatch("tâche, tentative ou résultat"))
        );
    }
}
