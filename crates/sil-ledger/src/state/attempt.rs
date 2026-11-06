//! Une tentative : claim retenu par le comité, bail, résultat, vérificateurs, verdicts.

use super::{
    index::{Ev, View},
    types::{AttemptState, ResultState, VerifierState},
    votes::{agreed_time, multi_winners, single_winner},
};
use crate::{Event, EventBody};
use sil_types::{Hash32, PublicKey};
use std::collections::BTreeSet;

/// Renouvellements de bail pris en compte.
const MAX_RENEWALS: usize = 10;
/// Vérificateurs acceptés au plus (2V-1 arbitrage compris, borné).
const MAX_VERIFIERS: usize = 31;

pub(super) struct Accepted<'a> {
    pub(super) claim_id: Hash32,
    pub(super) claim: &'a Event,
    pub(super) accepted_at_ms: u64,
}

fn find<'a>(events: &[Ev<'a>], id: &Hash32) -> Option<&'a Event> {
    events
        .iter()
        .copied()
        .find(|(hash, _)| *hash == id)
        .map(|(_, event)| event)
}

/// Claim retenu pour la tentative `k`, s'il y en a un.
pub(super) fn accepted<'a>(view: &View<'a>, events: &[Ev<'a>], k: u32) -> Option<Accepted<'a>> {
    let winner = single_winner(events, view.spec, |event| match &event.body {
        EventBody::Accept { attempt, claim, .. } if *attempt == k => Some(*claim),
        _ => None,
    })?;
    let claim = find(events, &winner.key)?;
    matches!(claim.body, EventBody::Claim { attempt, .. } if attempt == k).then(|| Accepted {
        claim_id: winner.key,
        claim,
        accepted_at_ms: agreed_time(&winner.votes, view.spec),
    })
}

pub(super) fn detail(
    view: &View<'_>,
    events: &[Ev<'_>],
    k: u32,
    won: &Accepted<'_>,
) -> AttemptState {
    let claimant = won.claim.author;
    let own = || {
        events
            .iter()
            .copied()
            .filter(move |(_, e)| e.author == claimant && e.body.attempt() == Some(k))
    };
    let renewals = own()
        .filter(|(_, e)| matches!(e.body, EventBody::Renew { .. }))
        .count()
        .min(MAX_RENEWALS);
    let result = own()
        .filter(|(_, e)| matches!(e.body, EventBody::Submit { .. }))
        .min_by_key(|(_, e)| e.sequence)
        .and_then(|(id, e)| match &e.body {
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
            }),
            _ => None,
        });
    let renewals_u32 = u32::try_from(renewals).unwrap_or(u32::MAX);
    let lease = view
        .spec
        .lease_ms
        .saturating_mul(u64::from(renewals_u32) + 1);
    AttemptState {
        attempt: k,
        claimant,
        claim_event: won.claim_id,
        accepted_at_ms: won.accepted_at_ms,
        renewals: renewals_u32,
        lease_expires_at_ms: won.accepted_at_ms.saturating_add(lease),
        released: own().any(|(_, e)| matches!(e.body, EventBody::Release { .. })),
        verifiers: verifiers(view, events, k, claimant, result.as_ref()),
        result,
    }
}

fn verifiers(
    view: &View<'_>,
    events: &[Ev<'_>],
    k: u32,
    claimant: PublicKey,
    result: Option<&ResultState>,
) -> Vec<VerifierState> {
    let winners = multi_winners(events, view.spec, |event| match &event.body {
        EventBody::VerifierAccept { attempt, offer, .. } if *attempt == k => Some(*offer),
        _ => None,
    });
    let mut chosen: Vec<(u64, Hash32, PublicKey)> = Vec::new();
    for winner in winners {
        let Some(offer) = find(events, &winner.key) else {
            continue;
        };
        let valid = matches!(offer.body, EventBody::VerifyOffer { attempt, .. } if attempt == k)
            && offer.author != claimant;
        if valid {
            chosen.push((
                agreed_time(&winner.votes, view.spec),
                winner.key,
                offer.author,
            ));
        }
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
            _ => None,
        })
        .min_by_key(|(sequence, _)| *sequence)
        .map(|(_, approved)| approved)
}
