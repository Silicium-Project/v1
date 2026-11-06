//! Offres en attente de décision : ce que le comité (ou son coordinateur) doit trancher.

use super::{
    index::{Ev, View},
    types::{OpenOffer, TaskState, TaskStatus},
};
use crate::{Event, EventBody, spec::JobSpec};
use sil_types::PublicKey;
use std::collections::{BTreeMap, BTreeSet};

/// Première offre de chaque auteur retenue par `keep`, triée par identifiant.
fn earliest_per_author(events: &[Ev<'_>], keep: impl Fn(&Event) -> bool) -> Vec<OpenOffer> {
    let mut best: BTreeMap<PublicKey, OpenOffer> = BTreeMap::new();
    let mut sequences: BTreeMap<PublicKey, u64> = BTreeMap::new();
    for (id, event) in events.iter().copied() {
        if !keep(event) {
            continue;
        }
        let earlier = sequences
            .get(&event.author)
            .is_none_or(|known| event.sequence < *known);
        if earlier {
            sequences.insert(event.author, event.sequence);
            best.insert(
                event.author,
                OpenOffer {
                    event: *id,
                    author: event.author,
                    issued_at_ms: event.issued_at_ms,
                },
            );
        }
    }
    let mut offers: Vec<OpenOffer> = best.into_values().collect();
    offers.sort_by_key(|offer| offer.event);
    offers
}

/// Claims pour la prochaine tentative, hors anciens claimants.
pub(super) fn open_claims(view: &View<'_>, state: &TaskState) -> Vec<OpenOffer> {
    let Some(next) = state.next_attempt else {
        return Vec::new();
    };
    let previous: BTreeSet<PublicKey> = state.attempts.iter().map(|a| a.claimant).collect();
    earliest_per_author(view.task_events(state.task.fragment), |event| {
        matches!(event.body, EventBody::Claim { attempt, .. } if attempt == next)
            && !previous.contains(&event.author)
    })
}

/// Propositions de vérification pour la dernière tentative, hors claimant et vérificateurs
/// déjà retenus. Vide si la tentative n'attend pas de vérification.
pub(super) fn open_verifications(view: &View<'_>, state: &TaskState) -> Vec<OpenOffer> {
    let Some(last) = state.attempts.last() else {
        return Vec::new();
    };
    if !matches!(state.status, TaskStatus::Completed | TaskStatus::Disputed) {
        return Vec::new();
    }
    let mut excluded: BTreeSet<PublicKey> = last.verifiers.iter().map(|v| v.verifier).collect();
    excluded.insert(last.claimant);
    earliest_per_author(view.task_events(state.task.fragment), |event| {
        matches!(event.body, EventBody::VerifyOffer { attempt, .. } if attempt == last.attempt)
            && !excluded.contains(&event.author)
    })
}

/// Places de vérificateur que le comité peut encore pourvoir.
pub(super) fn verification_slots_open(spec: &JobSpec, state: &TaskState, now_ms: u64) -> u32 {
    let Some(last) = state.attempts.last() else {
        return 0;
    };
    let needed = spec.verifications_required;
    let wanted = match state.status {
        TaskStatus::Completed => needed,
        TaskStatus::Disputed => needed.saturating_mul(2).saturating_sub(1),
        _ => return 0,
    };
    let active = last
        .verifiers
        .iter()
        .filter(|v| v.verdict.is_some() || now_ms <= v.accepted_at_ms.saturating_add(spec.lease_ms))
        .count();
    wanted.saturating_sub(u32::try_from(active).unwrap_or(u32::MAX))
}
