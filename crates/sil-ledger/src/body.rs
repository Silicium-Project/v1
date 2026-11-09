use crate::{LedgerError, SignedJobSpec};
use serde::{Deserialize, Serialize};
use sil_attest::{Attestation, Certificate};
use sil_types::{Hash32, JobId, TaskRef};

/// Plus grand numéro de tentative.
pub const MAX_ATTEMPTS: u32 = 16;

/// Ce que dit un événement. Qui peut l'émettre est vérifié à l'insertion
/// (`job.rs`) ; ce qu'il change est décidé par la dérivation (`state/`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventBody {
    /// Requester : crée le job (premier événement de sa chaîne).
    JobPublished { spec: SignedJobSpec },
    /// Tout pair : propose de calculer la tentative.
    Claim { fragment: u32, attempt: u32 },
    /// Claimant : prolonge le bail.
    Renew { fragment: u32, attempt: u32 },
    /// Claimant : abandonne la tentative.
    Release { fragment: u32, attempt: u32 },
    /// Claimant : publie le résultat.
    Submit {
        fragment: u32,
        attempt: u32,
        result: Hash32,
        size: u64,
        milestones_root: Hash32,
    },
    /// Tout pair sauf le claimant : propose de vérifier.
    VerifyOffer { fragment: u32, attempt: u32 },
    /// Vérificateur accepté : verdict signé.
    Attest { attestation: Attestation },
    /// Tout pair : certifie un résultat (ADR 0005, phases 3-4). `claim` et `submit` désignent le
    /// claim et le résultat de l'auteur ; `verifiers` désigne les offres des vérificateurs qui
    /// approuvent, et le certificat porte leurs approbations. La validité ne lit que ces
    /// événements : ni l'ordre des claims ni l'arrivée d'une offre plus ancienne ne la défont.
    Finalize {
        fragment: u32,
        attempt: u32,
        claim: Hash32,
        submit: Hash32,
        verifiers: Vec<Hash32>,
        certificate: Certificate,
    },
    /// Requester : annule le job.
    Cancel,
}

impl EventBody {
    /// Fragment visé, s'il y en a un.
    pub fn fragment(&self) -> Option<u32> {
        match self {
            Self::Claim { fragment, .. }
            | Self::Renew { fragment, .. }
            | Self::Release { fragment, .. }
            | Self::Submit { fragment, .. }
            | Self::VerifyOffer { fragment, .. }
            | Self::Finalize { fragment, .. } => Some(*fragment),
            Self::Attest { attestation } => Some(attestation.body.task.fragment),
            Self::JobPublished { .. } | Self::Cancel => None,
        }
    }

    /// Tentative visée, s'il y en a une.
    pub fn attempt(&self) -> Option<u32> {
        match self {
            Self::Claim { attempt, .. }
            | Self::Renew { attempt, .. }
            | Self::Release { attempt, .. }
            | Self::Submit { attempt, .. }
            | Self::VerifyOffer { attempt, .. }
            | Self::Finalize { attempt, .. } => Some(*attempt),
            Self::Attest { attestation } => Some(attestation.body.attempt),
            Self::JobPublished { .. } | Self::Cancel => None,
        }
    }

    /// Contrôles qui ne dépendent d'aucun autre événement.
    pub(crate) fn validate_shape(&self, job: &JobId) -> Result<(), LedgerError> {
        if let Some(attempt) = self.attempt()
            && !(1..=MAX_ATTEMPTS).contains(&attempt)
        {
            return Err(LedgerError::InvalidEvent("tentative hors bornes"));
        }
        match self {
            Self::Attest { attestation } if attestation.body.task.job != *job => {
                Err(LedgerError::InvalidEvent("attestation d'un autre job"))
            }
            Self::Finalize {
                fragment,
                attempt,
                certificate,
                ..
            } if certificate.task != TaskRef::new(*job, *fragment)
                || certificate.attempt != *attempt =>
            {
                Err(LedgerError::InvalidEvent("certificat incohérent"))
            }
            Self::Finalize { verifiers, .. } if verifiers.len() > sil_attest::MAX_APPROVALS => {
                Err(LedgerError::InvalidEvent("trop de vérificateurs désignés"))
            }
            _ => Ok(()),
        }
    }
}
