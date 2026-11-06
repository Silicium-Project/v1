//! Finalisation : vote du comité + certificat d'approbations recoupé avec le journal.
//!
//! Les signatures du certificat sont vérifiées une fois, à l'insertion (`job.rs`).
//! Ici on vérifie qu'il est cohérent avec ce que le journal sait : résultat, vérificateurs
//! acceptés par le comité, absence de rejets suffisants.

use super::{
    index::{Ev, View},
    types::{AttemptState, Finalized},
    votes::single_winner,
};
use crate::EventBody;
use sil_attest::Certificate;
use sil_types::PublicKey;
use std::collections::BTreeSet;

fn certificate_holds(
    required: usize,
    attempts: &[AttemptState],
    attempt: u32,
    certificate: &Certificate,
) -> bool {
    let Some(state) = attempt
        .checked_sub(1)
        .and_then(|index| attempts.get(index as usize))
    else {
        return false;
    };
    let Some(result) = &state.result else {
        return false;
    };
    if result.result != certificate.result || state.rejections() >= required {
        return false;
    }
    let approving: BTreeSet<&PublicKey> = state
        .verifiers
        .iter()
        .filter(|v| v.verdict == Some(true))
        .map(|v| &v.verifier)
        .collect();
    let backed: BTreeSet<&PublicKey> = certificate
        .approvals
        .iter()
        .map(|a| &a.verifier)
        .filter(|verifier| approving.contains(verifier))
        .collect();
    backed.len() >= required
}

pub(super) fn find(
    view: &View<'_>,
    events: &[Ev<'_>],
    attempts: &[AttemptState],
) -> Option<Finalized> {
    let required = usize::try_from(view.spec.verifications_required).unwrap_or(usize::MAX);
    let winner = single_winner(events, view.spec, |event| match &event.body {
        EventBody::Finalize {
            attempt,
            certificate,
            ..
        } if certificate_holds(required, attempts, *attempt, certificate) => {
            Some((*attempt, certificate.result))
        }
        _ => None,
    })?;
    let (attempt, result) = winner.key;
    let state = attempts.get(attempt.checked_sub(1)? as usize)?;
    Some(Finalized {
        attempt,
        result,
        claimant: state.claimant,
        approvers: state
            .verifiers
            .iter()
            .filter(|v| v.verdict == Some(true))
            .map(|v| v.verifier)
            .collect(),
    })
}
