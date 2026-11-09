//! Garde d'escrow : avant de travailler, le worker vérifie que l'escrow on-chain correspond au
//! `JobSpec` signé. Sans cela, un requester pourrait annoncer des récompenses dans
//! le spec, et ouvrir sur la chaîne un escrow aux conditions différentes.

use sil_chain_core::{Escrow, EscrowTerms, RuleError};
use sil_ledger::JobSpec;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GuardError {
    #[error("l'escrow n'appartient pas à ce job")]
    WrongJob,
    #[error("conditions de l'escrow différentes du spec : {0}")]
    TermsMismatch(&'static str),
    #[error("l'escrow est fermé (remboursé)")]
    Closed,
    #[error("escrow sous-financé : {have} < {need}")]
    Underfunded { have: u64, need: u64 },
    #[error("trop tard : l'échéance tombe avant la fin du travail")]
    TooLate,
    #[error("job impayable : workload sans règle d'unités ou entrée trop petite")]
    Unpayable,
    #[error("montants invalides")]
    Overflow,
}

/// Conditions d'escrow que le `JobSpec` impose.
pub fn terms_of(spec: &JobSpec) -> EscrowTerms {
    EscrowTerms {
        requester: spec.requester,
        workload: spec.workload.clone(),
        input_size: spec.input_size,
        width: spec.width,
        height: spec.height,
        fragments: spec.fragments,
        verifications_required: spec.verifications_required,
        reward_per_unit: spec.reward_per_unit,
        verifier_reward_per_unit: spec.verifier_reward_per_unit,
        expires_at_ms: spec.expires_at_ms,
    }
}

/// Vérifie qu'un worker peut travailler sur ce job en étant payé.
///
/// `min_remaining_ms` : temps dont il a besoin avant l'échéance.
pub fn check_escrow(
    spec: &JobSpec,
    escrow: &Escrow,
    now_ms: u64,
    min_remaining_ms: u64,
) -> Result<(), GuardError> {
    let job = spec.job_id().map_err(|_| GuardError::WrongJob)?;
    if escrow.job != job {
        return Err(GuardError::WrongJob);
    }
    let (want, have) = (terms_of(spec), &escrow.terms);
    let field = |same: bool, name: &'static str| {
        if same {
            Ok(())
        } else {
            Err(GuardError::TermsMismatch(name))
        }
    };
    field(want.requester == have.requester, "requester")?;
    field(want.workload == have.workload, "workload")?;
    field(want.input_size == have.input_size, "taille de l'entrée")?;
    field(want.width == have.width, "largeur")?;
    field(want.height == have.height, "hauteur")?;
    field(want.fragments == have.fragments, "fragments")?;
    field(
        want.verifications_required == have.verifications_required,
        "vérifications",
    )?;
    field(
        want.reward_per_unit == have.reward_per_unit,
        "tarif par unité",
    )?;
    field(
        want.verifier_reward_per_unit == have.verifier_reward_per_unit,
        "tarif vérificateur par unité",
    )?;
    field(want.expires_at_ms == have.expires_at_ms, "échéance")?;
    if escrow.is_closed() {
        return Err(GuardError::Closed);
    }
    let need = remaining_budget(&want, escrow)?;
    if escrow.balance() < need {
        return Err(GuardError::Underfunded {
            have: escrow.balance(),
            need,
        });
    }
    if now_ms.saturating_add(min_remaining_ms) > spec.expires_at_ms {
        return Err(GuardError::TooLate);
    }
    Ok(())
}

/// Fonds encore nécessaires : le budget de chaque fragment qui n'est pas encore réglé.
fn remaining_budget(terms: &EscrowTerms, escrow: &Escrow) -> Result<u64, GuardError> {
    (0..terms.fragments)
        .filter(|fragment| escrow.settled(*fragment).is_none())
        .try_fold(0u64, |total, fragment| {
            let budget = terms.fragment_budget(fragment).map_err(rule_error)?;
            total.checked_add(budget).ok_or(GuardError::Overflow)
        })
}

/// Un dépassement reste un dépassement ; tout autre refus des règles rend le job impayable.
fn rule_error(error: RuleError) -> GuardError {
    match error {
        RuleError::Overflow => GuardError::Overflow,
        _ => GuardError::Unpayable,
    }
}
