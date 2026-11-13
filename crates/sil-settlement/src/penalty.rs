//! Pénalité d'un vérificateur qui a voté contre la majorité (ADR 0012) : preuve, identifiant, et
//! file d'envoi vers la chaîne.
//!
//! La preuve est le verdict déjà signé par le vérificateur (`Domain::Attestation`). Tout nœud qui
//! la voit peut la soumettre ; le programme refuse les doublons (`déjà pénalisé`).

use crate::{Chain, ChainError, SettlementError, TxId, TxStatus};
use sil_chain_core::PenaltyArgs;
use sil_chain_core::wire::{penalty_frame, verdict_prefix};
use sil_crypto::{Domain, sha256, verify};
use sil_types::{Hash32, JobId, PublicKey, Signature};
use std::collections::BTreeMap;

/// Une pénalité prête à être envoyée : le verdict contraire, sa signature, et de quoi payer la
/// majorité et le requester.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenaltyClaim {
    pub job: JobId,
    pub requester: PublicKey,
    /// Majorité réglée : reçoit 30 % de la caution confisquée.
    pub approvers: Vec<PublicKey>,
    pub penalty: PenaltyArgs,
    /// Signature `Domain::Attestation` du verdict, par `penalty.verifier`.
    pub signature: Signature,
}

impl PenaltyClaim {
    /// Domaine de signature du verdict : tolérance ou exact.
    pub fn domain(&self) -> Domain {
        if self.penalty.tolerance.is_some() {
            Domain::AttestationTolerance
        } else {
            Domain::Attestation
        }
    }

    /// Octets signés (sans préfixe de domaine) : ceux du verdict, tels que `sil-attest` les signe.
    pub fn message(&self) -> Vec<u8> {
        let frame = penalty_frame(&self.job, &self.penalty);
        frame[verdict_prefix(self.penalty.tolerance.is_some()).len()..].to_vec()
    }

    /// Vrai si la signature du verdict est celle du vérificateur.
    pub fn is_signed(&self) -> bool {
        verify(
            &self.penalty.verifier,
            self.domain(),
            &self.message(),
            &self.signature,
        )
        .is_ok()
    }

    /// Identifiant unique : hash du verdict, de la signature et du vérificateur.
    pub fn id(&self) -> Hash32 {
        let mut bytes = self.message();
        bytes.extend_from_slice(self.penalty.verifier.as_bytes());
        bytes.extend_from_slice(self.signature.as_bytes());
        sha256(&bytes)
    }
}

/// Où en est une pénalité dans la file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PenaltyPhase {
    Pending,
    Submitted { tx: TxId },
    /// Lue sur la chaîne.
    Confirmed,
    /// Définitif : le programme refuse (déjà pénalisé, pas de compte, pas de preuve valide…).
    Rejected { reason: String },
}

/// File des pénalités à envoyer. Idempotente : une même preuve n'entre qu'une fois.
#[derive(Default)]
pub struct PenaltyBox {
    entries: BTreeMap<Hash32, (PenaltyClaim, PenaltyPhase)>,
}

impl PenaltyBox {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute une pénalité après avoir vérifié sa signature. Un doublon n'est pas ajouté.
    pub fn enqueue(&mut self, claim: PenaltyClaim) -> Result<Hash32, SettlementError> {
        if !claim.is_signed() {
            return Err(SettlementError::BadSignature);
        }
        let id = claim.id();
        self.entries
            .entry(id)
            .or_insert((claim, PenaltyPhase::Pending));
        Ok(id)
    }

    /// Fait avancer chaque pénalité d'un pas. Retourne le nombre de changements d'étape. Les
    /// erreurs transitoires laissent la pénalité en attente ; un refus du programme la clôt.
    pub fn advance(&mut self, chain: &dyn Chain) -> usize {
        let mut changed = 0;
        for (claim, phase) in self.entries.values_mut() {
            let next = match phase {
                PenaltyPhase::Pending => match chain.penalize(claim) {
                    Ok(tx) => Some(PenaltyPhase::Submitted { tx }),
                    Err(ChainError::Rejected(reason)) => Some(PenaltyPhase::Rejected { reason }),
                    Err(_) => None,
                },
                PenaltyPhase::Submitted { tx } => match chain.tx_status(tx) {
                    Ok(TxStatus::Confirmed) => Some(PenaltyPhase::Confirmed),
                    Ok(TxStatus::Failed(reason)) => Some(PenaltyPhase::Rejected { reason }),
                    Ok(TxStatus::Expired) => Some(PenaltyPhase::Pending),
                    Ok(TxStatus::Pending) | Err(_) => None,
                },
                PenaltyPhase::Confirmed | PenaltyPhase::Rejected { .. } => None,
            };
            if let Some(next) = next {
                *phase = next;
                changed += 1;
            }
        }
        changed
    }

    pub fn phase(&self, id: &Hash32) -> Option<&PenaltyPhase> {
        self.entries.get(id).map(|(_, phase)| phase)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
