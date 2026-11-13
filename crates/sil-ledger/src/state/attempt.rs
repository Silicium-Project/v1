//! Une tentative : claim retenu, bail, résultat, vérificateurs, verdicts.
//!
//! Sans vote (ADR 0005, phase 2) : la tentative `k` est retenue par le **premier claim valide**
//! dans l'ordre total (heure d'émission, identifiant). Pour `k > 1`, un claim n'est valide que si
//! la tentative `k-1` est libérée, rejetée, ou a un bail expiré avant l'émission du claim. Le
//! bail ne s'étend que par des renouvellements émis avant son échéance. Tout se décide sur les
//! heures des événements : aucune horloge locale n'intervient.

use super::{
    index::{Ev, View, by_time},
    types::{AttemptState, ResultState, VerifierState},
};
use crate::{Event, EventBody, spec::JobSpec};
use sil_types::{Hash32, PublicKey};
use std::collections::BTreeSet;

/// Renouvellements de bail pris en compte.
const MAX_RENEWALS: u32 = 10;
/// Vérificateurs acceptés au plus (2V-1 arbitrage compris, borné).
const MAX_VERIFIERS: usize = 31;

pub(super) struct Accepted<'a> {
    pub(super) claim_id: Hash32,
    pub(super) claim: &'a Event,
}

/// Bail d'une tentative, lu sur les seuls événements de son claimant pour cette tentative.
pub(super) struct Lease {
    pub(super) expires_at_ms: u64,
    pub(super) renewals: u32,
    pub(super) released: bool,
    pub(super) result: Option<ResultState>,
}

pub(super) fn find<'a>(events: &[Ev<'a>], id: &Hash32) -> Option<&'a Event> {
    events
        .iter()
        .copied()
        .find(|(hash, _)| *hash == id)
        .map(|(_, event)| event)
}

/// La tentative `prev` est-elle terminée à l'heure `at_ms` (émission d'un claim suivant) ?
/// Libérée, rejetée par les vérificateurs, ou bail expiré sans résultat à cette heure.
fn free_at(spec: &JobSpec, prev: &AttemptState, at_ms: u64) -> bool {
    if prev.released {
        return true;
    }
    if prev.result.is_some() {
        let needed = usize::try_from(spec.verifications_required).unwrap_or(usize::MAX);
        return prev.rejections() >= needed;
    }
    at_ms > prev.lease_expires_at_ms
}

/// Claim retenu pour la tentative `k` : le premier claim valide dans l'ordre total. `attempts`
/// sont les tentatives `1..k-1`, déjà retenues. Un ancien claimant du fragment ne peut pas le
/// reprendre : le réducteur le fait comme la politique de réclamation du nœud.
pub(super) fn accepted<'a>(
    view: &View<'a>,
    events: &[Ev<'a>],
    k: u32,
    attempts: &[AttemptState],
) -> Option<Accepted<'a>> {
    let previous = attempts.last();
    if k > 1 && previous.is_none() {
        return None;
    }
    let claimants: BTreeSet<PublicKey> = attempts.iter().map(|a| a.claimant).collect();
    let mut claims: Vec<Ev<'a>> = events
        .iter()
        .copied()
        .filter(|(_, event)| matches!(event.body, EventBody::Claim { attempt, .. } if attempt == k))
        .collect();
    claim_order(&mut claims);
    claims
        .into_iter()
        .find(|(_, claim)| {
            !claimants.contains(&claim.author)
                && previous.is_none_or(|p| free_at(view.spec, p, claim.issued_at_ms))
        })
        .map(|(id, claim)| Accepted {
            claim_id: *id,
            claim,
        })
}

/// Ordre des claims (ADR 0009) : slot d'ancrage signé, puis heure déclarée, puis identifiant. Le
/// slot d'ancrage est fixé par l'auteur à partir d'une horloge externe ; l'heure déclarée ne sert
/// qu'à départager deux claims dans le même slot.
pub(super) fn claim_order(events: &mut [Ev<'_>]) {
    events.sort_by_key(|(id, event)| {
        let anchor = match event.body {
            EventBody::Claim { anchor_slot, .. } => anchor_slot,
            _ => 0,
        };
        (anchor, event.issued_at_ms, **id)
    });
}

/// Résultat désigné par un `Submit`, si l'événement en est un.
pub(super) fn result_state(id: &Hash32, event: &Event) -> Option<ResultState> {
    match &event.body {
        EventBody::Submit {
            result,
            size,
            milestones_root,
            ..
        } => Some(ResultState {
            event: *id,
            result: *result,
            size: *size,
            milestones_root: *milestones_root,
            submitted_at_ms: event.issued_at_ms,
        }),
        _ => None,
    }
}

/// Bail de la tentative `k` du claimant, à partir de l'heure d'émission de son claim. Chaque
/// renouvellement émis avant l'échéance l'étend d'une période ; ce qui arrive après n'a pas
/// d'effet. Le premier `Submit` dans le bail est le résultat.
pub(super) fn own_lease(
    view: &View<'_>,
    events: &[Ev<'_>],
    k: u32,
    claimant: PublicKey,
    claimed_at_ms: u64,
) -> Lease {
    let lease_ms = view.spec.lease_ms;
    let mut own: Vec<Ev<'_>> = events
        .iter()
        .copied()
        .filter(|(_, e)| e.author == claimant && e.body.attempt() == Some(k))
        .collect();
    by_time(&mut own);
    let mut expires = claimed_at_ms.saturating_add(lease_ms);
    let mut renewals = 0;
    let mut result = None;
    let mut released = false;
    for (id, e) in own {
        if e.issued_at_ms > expires {
            continue;
        }
        match &e.body {
            EventBody::Renew { .. } if renewals < MAX_RENEWALS => {
                renewals += 1;
                expires = expires.saturating_add(lease_ms);
            }
            EventBody::Submit { .. } if result.is_none() => result = result_state(id, e),
            EventBody::Release { .. } => released = true,
            _ => {}
        }
    }
    Lease {
        expires_at_ms: expires,
        renewals,
        released,
        result,
    }
}

pub(super) fn detail(
    view: &View<'_>,
    events: &[Ev<'_>],
    k: u32,
    won: &Accepted<'_>,
) -> AttemptState {
    let claimant = won.claim.author;
    let lease = own_lease(view, events, k, claimant, won.claim.issued_at_ms);
    AttemptState {
        attempt: k,
        claimant,
        claim_event: won.claim_id,
        accepted_at_ms: won.claim.issued_at_ms,
        renewals: lease.renewals,
        lease_expires_at_ms: lease.expires_at_ms,
        released: lease.released,
        verifiers: verifiers(view, events, k, claimant, lease.result.as_ref()),
        result: lease.result,
    }
}

/// Vérificateurs retenus pour la tentative `k` du claimant, et leur verdict sur `result`.
pub(super) fn verifiers(
    view: &View<'_>,
    events: &[Ev<'_>],
    k: u32,
    claimant: PublicKey,
    result: Option<&ResultState>,
) -> Vec<VerifierState> {
    let mut chosen: Vec<(u64, Hash32, PublicKey)> = Vec::new();
    // Vérificateurs (ADR 0005, ADR 0006) : les k premières offres valides dans l'ordre total,
    // d'auteurs distincts, jamais le claimant. Aucune liste blanche : la réputation on-chain est
    // contrôlée au règlement. Aucun vote.
    let wanted = usize::try_from(view.spec.verifications_required).unwrap_or(usize::MAX);
    let mut offers: Vec<Ev<'_>> = events
        .iter()
        .copied()
        .filter(|(_, e)| {
            matches!(e.body, EventBody::VerifyOffer { attempt, .. } if attempt == k)
                && e.author != claimant
        })
        .collect();
    by_time(&mut offers);
    let mut distinct = BTreeSet::new();
    for (id, offer) in offers
        .into_iter()
        .filter(|(_, offer)| distinct.insert(offer.author))
        .take(wanted)
    {
        chosen.push((offer.issued_at_ms, *id, offer.author));
    }
    chosen.sort();
    let mut seen = BTreeSet::new();
    chosen
        .into_iter()
        .filter(|(_, _, author)| seen.insert(*author))
        .take(MAX_VERIFIERS)
        .map(|(accepted_at_ms, offer_event, verifier)| VerifierState {
            verifier,
            offer_event,
            accepted_at_ms,
            verdict: result.and_then(|r| verdict(events, &verifier, k, &r.result)),
        })
        .collect()
}

/// Premier verdict du vérificateur sur ce résultat précis.
fn verdict(events: &[Ev<'_>], verifier: &PublicKey, k: u32, result: &Hash32) -> Option<bool> {
    events
        .iter()
        .copied()
        .filter(|(_, e)| e.author == *verifier)
        .filter_map(|(_, e)| match &e.body {
            EventBody::Attest { attestation }
                if attestation.body.attempt == k && attestation.body.result == *result =>
            {
                Some((e.sequence, attestation.body.approved))
            }
            EventBody::AttestTolerance { attestation }
                if attestation.body.attempt == k && attestation.body.result == *result =>
            {
                Some((e.sequence, attestation.body.approved))
            }
            _ => None,
        })
        .min_by_key(|(sequence, _)| *sequence)
        .map(|(_, approved)| approved)
}
