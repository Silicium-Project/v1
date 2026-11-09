use crate::AttestError;
use serde::{Deserialize, Serialize};
use sil_codec::encode;
use sil_crypto::{Domain, SigningIdentity, verify};
use sil_types::{Hash32, PublicKey, Signature, TaskRef};

/// Contenu signé d'une attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationBody {
    pub task: TaskRef,
    pub attempt: u32,
    /// Hash du résultat jugé.
    pub result: Hash32,
    pub approved: bool,
    /// Hash de la preuve de divergence (obligatoire pour un rejet, interdite sinon).
    pub evidence: Option<Hash32>,
}

impl AttestationBody {
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

/// Verdict signé d'un vérificateur. Une approbation porte en plus la signature de règlement du
/// vérificateur (ADR 0007) : elle autorise le paiement on-chain du fragment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    pub body: AttestationBody,
    pub verifier: PublicKey,
    pub signature: Signature,
    /// Signature `Domain::Settlement` du message de règlement, pour une approbation.
    pub settlement: Option<Signature>,
}

impl Attestation {
    /// Signe un verdict avec l'identité du vérificateur.
    pub fn sign(identity: &SigningIdentity, body: AttestationBody) -> Result<Self, AttestError> {
        body.check_shape()?;
        let signature = identity.sign(Domain::Attestation, &body.bytes()?);
        Ok(Self {
            body,
            verifier: identity.public(),
            signature,
            settlement: None,
        })
    }

    /// Ajoute la signature de règlement d'une approbation. `message` : job ‖ fragment ‖ tentative
    /// ‖ résultat ‖ claimant (voir `sil-chain-core`). Sans effet sur un rejet.
    pub fn with_settlement(mut self, identity: &SigningIdentity, message: &[u8]) -> Self {
        if self.body.approved {
            self.settlement = Some(identity.sign(Domain::Settlement, message));
        }
        self
    }

    /// Vérifie la forme et la signature du verdict.
    pub fn verify(&self) -> Result<(), AttestError> {
        self.body.check_shape()?;
        verify(
            &self.verifier,
            Domain::Attestation,
            &self.body.bytes()?,
            &self.signature,
        )
        .map_err(|_| AttestError::BadSignature)
    }

    /// Vérifie la signature de règlement contre `message`. Erreur si elle manque.
    pub fn verify_settlement(&self, message: &[u8]) -> Result<(), AttestError> {
        let signature = self.settlement.as_ref().ok_or(AttestError::BadSignature)?;
        verify(&self.verifier, Domain::Settlement, message, signature)
            .map_err(|_| AttestError::BadSignature)
    }
}
