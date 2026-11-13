//! Construction de la finalisation et du règlement d'un fragment (ADR 0005 phase 3, ADR 0007).
//!
//! Tout nœud peut publier le `Finalize` d'un résultat approuvé par `k` vérificateurs retenus. Le
//! règlement on-chain est signé par ces mêmes approbateurs : chacun a signé `Domain::Settlement`
//! quand il a approuvé (voir `sil_attest::Attestation::with_settlement`).

use sil_attest::{Certificate, MAX_APPROVALS, ToleranceCertificate};
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
    if tolerance_mode(ledger, job) {
        let mut approvals = ledger.tolerance_approvals(job, fragment, last.attempt, now_ms);
        approvals.truncate(MAX_APPROVALS);
        if approvals.len() < required as usize {
            return None;
        }
        let verifiers: Vec<Hash32> = approvals
            .iter()
            .filter_map(|approval| offer_of(ledger, job, fragment, last.attempt, approval.verifier))
            .collect();
        return Some(EventBody::FinalizeTolerance {
            fragment,
            attempt: last.attempt,
            claim: last.claim_event,
            submit: submitted.event,
            verifiers,
            certificate: ToleranceCertificate {
                task: TaskRef::new(*job, fragment),
                attempt: last.attempt,
                result: submitted.result,
                approvals,
            },
        });
    }
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
    // Les signatures de règlement viennent des approbations du mode du workload.
    let settlements: Vec<_> = if tolerance_mode(ledger, job) {
        ledger
            .tolerance_approvals(job, fragment, finalized.attempt, now_ms)
            .into_iter()
            .filter_map(|approval| approval.settlement.map(|s| (approval.verifier, s)))
            .collect()
    } else {
        ledger
            .approvals(job, fragment, finalized.attempt, now_ms)
            .into_iter()
            .filter_map(|approval| approval.settlement.map(|s| (approval.verifier, s)))
            .collect()
    };
    for (verifier, signature) in settlements {
        let _ = auth.add(verifier, signature);
    }
    Some(auth)
}

/// Le workload de ce job est-il en mode tolérance ? Le mode vient du JobSpec signé.
fn tolerance_mode(ledger: &Ledger, job: &JobId) -> bool {
    ledger
        .spec(job)
        .is_some_and(|signed| sil_ledger::tolerance_rules(&signed.spec.workload).is_some())
}

/// Verdicts contraires à la majorité réglée sur `fragment` (ADR 0012), émis par les vérificateurs
/// retenus pour la tentative réglée. Vide tant que rien n'est réglé.
pub fn penalty_claims(
    ledger: &Ledger,
    job: &JobId,
    fragment: u32,
    now_ms: u64,
) -> Vec<sil_settlement::PenaltyClaim> {
    let Some(signed) = ledger.spec(job) else {
        return Vec::new();
    };
    let required = signed.spec.verifications_required;
    let requester = signed.spec.requester;
    let Some(state) = ledger.task_state(job, fragment, now_ms) else {
        return Vec::new();
    };
    let Some(finalized) = state.finalized.clone() else {
        return Vec::new();
    };
    let Some(claim) = SettlementClaim::from_finalized(job, fragment, &finalized, required) else {
        return Vec::new();
    };
    let Some(attempt) = state
        .attempts
        .iter()
        .find(|a| a.attempt == finalized.attempt)
    else {
        return Vec::new();
    };
    // Règles de tolérance du workload : modèle et runtime attendus (None : mode exact).
    let expected = sil_ledger::tolerance_rules(&signed.spec.workload);
    let mut out = Vec::new();
    // Seuls les vérificateurs retenus pour la tentative peuvent être pénalisés.
    for chosen in &attempt.verifiers {
        if claim.approvers.contains(&chosen.verifier) {
            continue;
        }
        for event in ledger.events_of(job, &chosen.verifier) {
            // Verdict exact ou tolérance : même test de contrariété, même preuve, même signature.
            let (task, verdict_attempt, result, approved, evidence, signature, tolerance) =
                match &event.body {
                    EventBody::Attest { attestation } => {
                        let b = &attestation.body;
                        (
                            b.task,
                            b.attempt,
                            b.result,
                            b.approved,
                            b.evidence,
                            attestation.signature.clone(),
                            None,
                        )
                    }
                    EventBody::AttestTolerance { attestation } => {
                        let b = &attestation.body;
                        // Un verdict au mauvais modèle ou runtime n'est pas celui du job.
                        match expected {
                            Some((model, runtime, _)) if b.model == model && b.runtime == runtime => {}
                            _ => continue,
                        }
                        (
                            b.task,
                            b.attempt,
                            b.result,
                            b.approved,
                            b.evidence,
                            attestation.signature.clone(),
                            Some(sil_chain_core::ToleranceProof {
                                model: b.model,
                                runtime: b.runtime,
                                sample_root: b.sample_root,
                                min_cos_micro: b.min_cos_micro,
                            }),
                        )
                    }
                    _ => continue,
                };
            if task.fragment != fragment || verdict_attempt != finalized.attempt {
                continue;
            }
            // Contraire : approuver un autre résultat, ou rejeter le résultat réglé (ADR 0012).
            let against = if approved {
                result != finalized.result
            } else {
                result == finalized.result
            };
            if !against {
                continue;
            }
            out.push(sil_settlement::PenaltyClaim {
                job: *job,
                requester,
                approvers: claim.approvers.clone(),
                penalty: sil_chain_core::PenaltyArgs {
                    fragment,
                    attempt: verdict_attempt,
                    result,
                    approved,
                    evidence,
                    tolerance,
                    verifier: chosen.verifier,
                },
                signature,
            });
        }
    }
    out
}
