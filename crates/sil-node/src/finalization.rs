//! Construction de la finalisation et du règlement d'un fragment (ADR 0005 phase 3, ADR 0007).
//!
//! Tout nœud peut publier le `Finalize` d'un résultat approuvé par `k` vérificateurs retenus. Le
//! règlement on-chain est signé par ces mêmes approbateurs : chacun a signé `Domain::Settlement`
//! quand il a approuvé (voir `sil_attest::Attestation::with_settlement`).

use sil_attest::{Certificate, MAX_APPROVALS};
use sil_ledger::{EventBody, Ledger, TaskStatus};
use sil_settlement::{Authorization, SettlementClaim};
use sil_types::{Hash32, JobId, PublicKey, TaskRef};

/// Identifiant de la première offre de vérification de `verifier` pour cette tentative.
fn offer_of(
    ledger: &Ledger,
    job: &JobId,
    fragment: u32,
    attempt: u32,
    verifier: PublicKey,
) -> Option<Hash32> {
    ledger
        .events_of(job, &verifier)
        .into_iter()
        .find(|e| {
            matches!(e.body, EventBody::VerifyOffer { fragment: f, attempt: a }
                if f == fragment && a == attempt)
        })
        .and_then(|e| e.id().ok())
}

/// `Finalize` à publier maintenant pour `fragment`, si le résultat est approuvé par assez de
/// vérificateurs retenus. `None` si le résultat n'est pas vérifié, ou sans approbations suffisantes.
pub fn finalization(ledger: &Ledger, job: &JobId, fragment: u32, now_ms: u64) -> Option<EventBody> {
    let required = ledger.spec(job)?.spec.verifications_required;
    let state = ledger.task_state(job, fragment, now_ms)?;
    if state.status != TaskStatus::Verified {
        return None;
    }
    let last = state.attempts.last()?;
    let submitted = last.result.as_ref()?;
    let mut approvals = ledger.approvals(job, fragment, last.attempt, now_ms);
    approvals.truncate(MAX_APPROVALS);
    if approvals.len() < required as usize {
        return None;
    }
    let verifiers: Vec<Hash32> = approvals
        .iter()
        .filter_map(|approval| offer_of(ledger, job, fragment, last.attempt, approval.verifier))
        .collect();
    Some(EventBody::Finalize {
        fragment,
        attempt: last.attempt,
        claim: last.claim_event,
        submit: submitted.event,
        verifiers,
        certificate: Certificate {
            task: TaskRef::new(*job, fragment),
            attempt: last.attempt,
            result: submitted.result,
            approvals,
        },
    })
}

/// Autorisation de règlement d'un fragment finalisé : les signatures de règlement des
/// approbateurs (ADR 0007). `None` tant que le journal n'a pas assez d'approbations signées.
pub fn authorization(
    ledger: &Ledger,
    job: &JobId,
    fragment: u32,
    now_ms: u64,
) -> Option<Authorization> {
    let required = ledger.spec(job)?.spec.verifications_required;
    let finalized = ledger.task_state(job, fragment, now_ms)?.finalized?;
    let claim = SettlementClaim::from_finalized(job, fragment, &finalized, required)?;
    let mut auth = Authorization::new(claim.clone());
    for approval in ledger.approvals(job, fragment, finalized.attempt, now_ms) {
        if let Some(signature) = approval.settlement {
            let _ = auth.add(approval.verifier, signature);
        }
    }
    Some(auth)
}
