//! Finalisation (ADR 0005, phases 3-4) : un `Finalize` certifie un résultat, sans vote.
//!
//! Un `Finalize` ne lit que ce qu'il désigne : son claim, son résultat (`Submit`), les offres des
//! vérificateurs qui approuvent (`verifiers`) et leurs attestations. Le bail se lit sur le claim
//! désigné. Rien ne dépend de l'ordre des offres ni du vainqueur de la course aux claims : une
//! offre plus ancienne reçue tard ne défait pas une finalisation.
//!
//! Validité monotone : une fois valide, elle le reste quand d'autres événements arrivent. Un rejet
//! postérieur ouvre un litige, il ne défait pas la finalisation. Entre plusieurs `Finalize` valides,
//! le premier dans l'ordre total l'emporte.

use super::{
    attempt,
    index::{Ev, View, by_time},
    types::Finalized,
};
use crate::EventBody;
use sil_types::{Hash32, PublicKey};
use std::collections::BTreeSet;

pub(super) fn find(view: &View<'_>, events: &[Ev<'_>]) -> Option<Finalized> {
    let required = usize::try_from(view.spec.verifications_required).unwrap_or(usize::MAX);
    let mut finals: Vec<Ev<'_>> = events
        .iter()
        .copied()
        .filter(|(_, e)| {
            matches!(
                e.body,
                EventBody::Finalize { .. } | EventBody::FinalizeTolerance { .. }
            )
        })
        .collect();
    by_time(&mut finals);
    finals.into_iter().find_map(|(_, event)| match &event.body {
        EventBody::Finalize {
            fragment,
            attempt,
            claim,
            submit,
            verifiers,
            certificate,
        } => {
            let facts: Vec<Fact> = certificate
                .approvals
                .iter()
                .map(|a| Fact {
                    verifier: a.verifier,
                    fragment: a.body.task.fragment,
                    attempt: a.body.attempt,
                    result: a.body.result,
                    approved: a.body.approved,
                })
                .collect();
            certified(
                view,
                events,
                Designated {
                    fragment: *fragment,
                    attempt: *attempt,
                    claim: *claim,
                    submit: *submit,
                    verifiers,
                },
                certificate.result,
                &facts,
                required,
            )
        }
        EventBody::FinalizeTolerance {
            fragment,
            attempt,
            claim,
            submit,
            verifiers,
            certificate,
        } => {
            let facts: Vec<Fact> = certificate
                .approvals
                .iter()
                .map(|a| Fact {
                    verifier: a.verifier,
                    fragment: a.body.task.fragment,
                    attempt: a.body.attempt,
                    result: a.body.result,
                    approved: a.body.approved,
                })
                .collect();
            certified(
                view,
                events,
                Designated {
                    fragment: *fragment,
                    attempt: *attempt,
                    claim: *claim,
                    submit: *submit,
                    verifiers,
                },
                certificate.result,
                &facts,
                required,
            )
        }
        _ => None,
    })
}

/// Une approbation réduite à ce que reste la finalisation : qui, sur quoi, et quel verdict.
/// Valable pour le mode exact comme pour le mode tolérance (la vérification fine est faite à la
/// réception de l'événement).
struct Fact {
    verifier: PublicKey,
    fragment: u32,
    attempt: u32,
    result: Hash32,
    approved: bool,
}

/// Ce que désigne un `Finalize`.
struct Designated<'a> {
    fragment: u32,
    attempt: u32,
    claim: Hash32,
    submit: Hash32,
    verifiers: &'a [Hash32],
}

/// Le certificat tient-il sur les événements désignés ?
fn certified(
    view: &View<'_>,
    events: &[Ev<'_>],
    designated: Designated<'_>,
    certified_result: Hash32,
    facts: &[Fact],
    required: usize,
) -> Option<Finalized> {
    let Designated {
        fragment,
        attempt: k,
        claim: claim_id,
        submit: submit_id,
        verifiers: offer_ids,
    } = designated;
    let claim = attempt::find(events, &claim_id)?;
    if !matches!(claim.body, EventBody::Claim { fragment: f, attempt: a } if f == fragment && a == k)
    {
        return None;
    }
    let submit = attempt::find(events, &submit_id)?;
    let EventBody::Submit {
        fragment: submitted_fragment,
        attempt: submitted_attempt,
        result,
        ..
    } = &submit.body
    else {
        return None;
    };
    if *submitted_fragment != fragment
        || *submitted_attempt != k
        || *result != certified_result
        || submit.author != claim.author
        || submit.issued_at_ms < claim.issued_at_ms
    {
        return None;
    }
    let claimant = claim.author;
    let lease = attempt::own_lease(view, events, k, claimant, claim.issued_at_ms);
    if submit.issued_at_ms > lease.expires_at_ms {
        return None;
    }
    // Vérificateurs désignés : offres valides, pour cette tentative, hors claimant (ADR 0006 : pas
    // de liste blanche). Une offre désignée qui ne tient pas est simplement sans effet.
    attempt::result_state(&submit_id, submit)?;
    let designated: BTreeSet<PublicKey> = offer_ids
        .iter()
        .filter_map(|id| attempt::find(events, id).map(|offer| (*id, offer)))
        .filter(|(_, offer)| {
            matches!(offer.body, EventBody::VerifyOffer { fragment: f, attempt: a } if f == fragment && a == k)
                && offer.author != claimant
        })
        .map(|(_, offer)| offer.author)
        .collect();
    let approvers: BTreeSet<PublicKey> = facts
        .iter()
        .filter(|a| a.fragment == fragment && a.attempt == k && a.result == *result && a.approved)
        .map(|a| a.verifier)
        .filter(|verifier| designated.contains(verifier))
        .collect();
    (approvers.len() >= required).then(|| Finalized {
        attempt: k,
        result: *result,
        claimant,
        approvers: approvers.into_iter().collect(),
    })
}
