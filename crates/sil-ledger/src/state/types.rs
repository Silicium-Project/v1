use serde::Serialize;
use sil_types::{Hash32, JobId, PublicKey, TaskRef};

/// Statut d'une tâche (fragment), dérivé des événements et de l'heure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TaskStatus {
    /// Aucune tentative acceptée.
    Open,
    /// Une tentative tourne sous bail valide.
    Claimed,
    LeaseExpired,
    Released,
    /// Résultat publié, vérifications insuffisantes.
    Completed,
    /// Assez d'approbations indépendantes.
    Verified,
    /// Assez d'approbations **et** assez de rejets : arbitrage requis.
    Disputed,
    /// Assez de rejets.
    Rejected,
    /// Dernière tentative échouée.
    Failed,
    Finalized,
    Cancelled,
    /// Date limite du job dépassée.
    Expired,
}

impl TaskStatus {
    /// La tentative a échoué : une suivante peut être acceptée.
    pub fn is_failed_attempt(self) -> bool {
        matches!(self, Self::LeaseExpired | Self::Released | Self::Rejected)
    }

    /// Plus aucun événement ne peut changer la tâche.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Finalized | Self::Cancelled | Self::Failed | Self::Expired
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResultState {
    pub event: Hash32,
    pub result: Hash32,
    pub size: u64,
    pub milestones_root: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerifierState {
    pub verifier: PublicKey,
    pub offer_event: Hash32,
    pub accepted_at_ms: u64,
    /// `Some(true)` approuvé, `Some(false)` rejeté, `None` pas encore de verdict.
    pub verdict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AttemptState {
    pub attempt: u32,
    pub claimant: PublicKey,
    pub claim_event: Hash32,
    /// Heure convenue du vote décisif du comité (le `threshold`-ième plus petit).
    pub accepted_at_ms: u64,
    pub renewals: u32,
    pub lease_expires_at_ms: u64,
    pub released: bool,
    pub result: Option<ResultState>,
    pub verifiers: Vec<VerifierState>,
}

impl AttemptState {
    pub(crate) fn approvals(&self) -> usize {
        self.verifiers
            .iter()
            .filter(|v| v.verdict == Some(true))
            .count()
    }

    pub(crate) fn rejections(&self) -> usize {
        self.verifiers
            .iter()
            .filter(|v| v.verdict == Some(false))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finalized {
    pub attempt: u32,
    pub result: Hash32,
    pub claimant: PublicKey,
    /// Vérificateurs ayant approuvé (à payer en phase 5).
    pub approvers: Vec<PublicKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskState {
    pub task: TaskRef,
    pub status: TaskStatus,
    pub attempts: Vec<AttemptState>,
    /// Tentative pour laquelle les claims sont les bienvenus.
    pub next_attempt: Option<u32>,
    pub finalized: Option<Finalized>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum JobStatus {
    Active,
    Finalized,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobState {
    pub job: JobId,
    pub status: JobStatus,
    pub tasks: Vec<TaskState>,
}

/// Offre (claim ou proposition de vérification) en attente de décision du comité.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenOffer {
    pub event: Hash32,
    pub author: PublicKey,
    pub issued_at_ms: u64,
}
