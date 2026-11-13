use crate::{RuleError, units};
use serde::{Deserialize, Serialize};
use sil_types::PublicKey;

pub const MAX_APPROVERS: usize = 16;
/// Plus grand quorum accepté : impair (ADR 0012).
pub const MAX_QUORUM: usize = 15;
const MAX_FRAGMENTS: u32 = 1024;

/// Délai de grâce entre l'échéance et le remboursement (24 h).
pub const REFUND_GRACE_MS: u64 = 24 * 3600 * 1000;

/// Frais versés à celui qui soumet un règlement (lamports). Financés par le requester au dépôt,
/// donc payés par le job et non par les nœuds. Couvrent les frais de transaction Solana et une marge.
pub const SETTLEMENT_FEE: u64 = 10_000;

/// Conditions du job, recopiées du `JobSpec` signé (le nœud vérifie qu'elles coïncident :
/// `sil-settlement::guard`). Les tarifs sont **par unité** : le montant d'un fragment dépend de
/// ses unités (`units`), fixées par le workload et la taille de l'entrée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowTerms {
    pub requester: PublicKey,
    /// Workload : fixe la règle d'unités (voir `units`).
    pub workload: String,
    /// Taille de l'entrée, en octets.
    pub input_size: u64,
    /// Image (`rendu/raytracer`) : largeur et hauteur en pixels, 0 sinon.
    pub width: u32,
    pub height: u32,
    pub fragments: u32,
    pub verifications_required: u32,
    /// Payé au claimant, pour chaque unité de son fragment.
    pub reward_per_unit: u64,
    /// Payé à chacun des `verifications_required` vérificateurs, pour chaque unité du fragment.
    pub verifier_reward_per_unit: u64,
    pub expires_at_ms: u64,
}

impl EscrowTerms {
    pub fn validate(&self) -> Result<(), RuleError> {
        if self.fragments == 0 || self.fragments > MAX_FRAGMENTS {
            return Err(RuleError::BadTerms("fragments"));
        }
        // k impair (ADR 0012) : une majorité stricte existe toujours, sans égalité. Le plafond est
        // donc 15, pas `MAX_APPROVERS` (16, pair).
        let v = self.verifications_required as usize;
        if v == 0 || v > MAX_QUORUM || v % 2 == 0 {
            return Err(RuleError::BadTerms("verifications_required"));
        }
        // Passe aussi la règle d'unités : workload inconnu ou entrée trop petite sont refusés.
        self.max_payout().map(|_| ())
    }

    /// Unités de travail du fragment `index`, selon le workload.
    pub fn fragment_units(&self, index: u32) -> Result<u64, RuleError> {
        let shape = units::Shape {
            input_size: self.input_size,
            width: self.width,
            height: self.height,
        };
        units::fragment_units(&self.workload, shape, self.fragments, index)
    }

    /// Budget d'un fragment : unités × (tarif du claimant + V × tarif d'un vérificateur),
    /// plus les frais de règlement.
    pub fn fragment_budget(&self, index: u32) -> Result<u64, RuleError> {
        let per_unit = u64::from(self.verifications_required)
            .checked_mul(self.verifier_reward_per_unit)
            .and_then(|verifiers| verifiers.checked_add(self.reward_per_unit))
            .ok_or(RuleError::Overflow)?;
        self.fragment_units(index)?
            .checked_mul(per_unit)
            .and_then(|rewards| rewards.checked_add(SETTLEMENT_FEE))
            .ok_or(RuleError::Overflow)
    }

    /// Total à déposer pour couvrir tous les fragments.
    pub fn max_payout(&self) -> Result<u64, RuleError> {
        (0..self.fragments).try_fold(0u64, |total, index| {
            total
                .checked_add(self.fragment_budget(index)?)
                .ok_or(RuleError::Overflow)
        })
    }
}
