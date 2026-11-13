//! Pénalité d'un vérificateur qui a voté contre la majorité d'un fragment (ADR 0012).
//!
//! La pénalité n'est pas une décision du requester. Elle repose sur une preuve : la signature du
//! vérificateur sur un verdict qui contredit le résultat réglé (`attestation_frame`). Sans cette
//! signature, rien n'est confisqué. La caution confisquée (ADR 0010) se répartit ainsi : 70 % au
//! requester, 30 % à parts égales entre les approbateurs (la majorité). Le reste d'arrondi revient
//! au requester.

use crate::{Escrow, Payout, RuleError, reputation, reserve};
use serde::{Deserialize, Serialize};
use sil_types::{Hash32, PublicKey};

/// Part de la caution confisquée qui va à la majorité, en pourcentage (le reste au requester).
pub const MAJORITY_SHARE_PERCENT: u128 = 30;

/// Champs propres à un verdict en mode tolérance (embedding) : modèle, runtime, racine
/// d'échantillon et cosinus minimal observé, en millionièmes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToleranceProof {
    pub model: Hash32,
    pub runtime: Hash32,
    pub sample_root: Hash32,
    pub min_cos_micro: i32,
}

/// Paramètres d'une pénalité : le verdict signé du vérificateur, et le vérificateur qui l'a signé.
/// Un verdict est contraire à la majorité s'il approuve un autre résultat que le résultat réglé, ou
/// s'il rejette le résultat réglé (avec sa preuve).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PenaltyArgs {
    pub fragment: u32,
    pub attempt: u32,
    /// Résultat jugé par le verdict.
    pub result: Hash32,
    pub approved: bool,
    /// Preuve de divergence : obligatoire pour un rejet, interdite pour une approbation.
    pub evidence: Option<Hash32>,
    /// `Some` si le verdict est en mode tolérance (signé sous `Domain::AttestationTolerance`).
    pub tolerance: Option<ToleranceProof>,
    pub verifier: PublicKey,
}

/// Effet d'une pénalité acceptée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Penalty {
    /// Nouveau score de réputation du vérificateur.
    pub score: i64,
    /// Montant prélevé sur la réserve du vérificateur.
    pub confiscated: u64,
    /// Versements : d'abord le requester, puis chaque approbateur.
    pub payouts: Vec<Payout>,
}

impl Escrow {
    /// Part de réserve (ADR 0010) retenue sur chaque vérificateur de ce fragment : le plafond de ce
    /// qu'une pénalité sur ce fragment peut confisquer. La réserve d'un vérificateur mêle plusieurs
    /// jobs ; une pénalité ne doit pas prendre ce qui vient d'un autre.
    pub fn held_for(&self, fragment: u32) -> Result<u64, RuleError> {
        let units = self.terms.fragment_units(fragment)?;
        let amount = units
            .checked_mul(self.terms.verifier_reward_per_unit)
            .ok_or(RuleError::Overflow)?;
        Ok(reserve::split(amount).1)
    }

    /// Pénalise `args.verifier`, qui a signé le verdict contraire décrit par `args` : `signed` sont les
    /// clés dont la signature du vote a été vérifiée sur la chaîne. `standing` : score actuel (`None`
    /// : pas de compte). `reserve` : réserve actuelle du vérificateur.
    ///
    /// Règles : le fragment est réglé, la tentative est celle du règlement, le vote diffère du
    /// résultat réglé, le vérificateur n'est pas dans la majorité et a signé ce vote, et il n'est
    /// pas déjà pénalisé pour ce fragment.
    pub fn penalize(
        &mut self,
        args: &PenaltyArgs,
        signed: &[PublicKey],
        standing: Option<i64>,
        reserve: u64,
    ) -> Result<Penalty, RuleError> {
        if self.closed {
            return Err(RuleError::Refunded);
        }
        let settled = self
            .settled
            .get(&args.fragment)
            .ok_or(RuleError::BadPenalty("fragment non réglé"))?;
        if args.attempt != settled.attempt {
            return Err(RuleError::BadPenalty("tentative différente du règlement"));
        }
        match (args.approved, args.evidence.is_some()) {
            (true, true) => return Err(RuleError::BadPenalty("preuve inattendue")),
            (false, false) => return Err(RuleError::BadPenalty("preuve manquante")),
            _ => {}
        }
        // Contraire : approuver un autre résultat, ou rejeter le résultat réglé.
        let against = if args.approved {
            args.result != settled.result
        } else {
            args.result == settled.result
        };
        if !against {
            return Err(RuleError::BadPenalty("verdict conforme au règlement"));
        }
        if settled.approvers.contains(&args.verifier) {
            return Err(RuleError::BadPenalty("vérificateur dans la majorité"));
        }
        if !signed.contains(&args.verifier) {
            return Err(RuleError::BadPenalty("vote contraire non signé"));
        }
        if self.penalized.contains(&(args.fragment, args.verifier)) {
            return Err(RuleError::BadPenalty("déjà pénalisé pour ce fragment"));
        }
        let score = standing.ok_or(RuleError::BadPenalty("pas de compte de réputation"))?;

        let confiscated = self.held_for(args.fragment)?.min(reserve);
        let majority = u64::try_from(
            u128::from(confiscated) * MAJORITY_SHARE_PERCENT / 100,
        )
        .map_err(|_| RuleError::Overflow)?;
        let approvers = settled.approvers.clone();
        let each = if approvers.is_empty() {
            0
        } else {
            majority / approvers.len() as u64
        };
        let to_requester = confiscated - each * approvers.len() as u64;

        let mut payouts = vec![Payout {
            to: self.terms.requester,
            amount: to_requester,
        }];
        if each > 0 {
            payouts.extend(approvers.iter().map(|to| Payout {
                to: *to,
                amount: each,
            }));
        }

        self.penalized.insert((args.fragment, args.verifier));
        Ok(Penalty {
            score: reputation::penalized(score),
            confiscated,
            payouts,
        })
    }
}
