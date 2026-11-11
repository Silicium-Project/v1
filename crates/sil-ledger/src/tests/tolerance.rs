//! Bout en bout du mode tolérance : verdict cosinus, certificat, finalisation, et refus des
//! événements qui ne respectent pas le descripteur du workload.

use super::support::*;
use crate::*;
use sil_attest::{ToleranceAttestation, ToleranceBody, ToleranceCertificate};
use sil_types::{Hash32, TaskRef};

const EMBED: &str = "embedding/e5-small";

fn embed_params() -> Params {
    Params {
        workload: EMBED.into(),
        ..Params::default()
    }
}

fn pins() -> (Hash32, Hash32, i32) {
    crate::tolerance::rules_of(EMBED).expect("le descripteur embedding est en mode tolérance")
}

fn verdict(
    world: &World,
    verifier: &sil_crypto::SigningIdentity,
    result: Hash32,
    model: Hash32,
    runtime: Hash32,
    min_cos_micro: i32,
) -> ToleranceAttestation {
    ToleranceAttestation::sign(
        verifier,
        ToleranceBody {
            task: TaskRef::new(world.job, 0),
            attempt: 1,
            result,
            model,
            runtime,
            sample_root: hash(5),
            min_cos_micro,
            approved: true,
            evidence: None,
        },
    )
    .unwrap()
}

fn offer_of(world: &World, verifier: &sil_crypto::SigningIdentity) -> Hash32 {
    world.task().attempts[0]
        .verifiers
        .iter()
        .find(|v| v.verifier == verifier.public())
        .map(|v| v.offer_event)
        .expect("vérificateur accepté")
}

/// Un worker publie `result`, un vérificateur est accepté. Retourne (monde, worker, vérificateur).
fn accepted() -> (
    World,
    sil_crypto::SigningIdentity,
    sil_crypto::SigningIdentity,
    Hash32,
) {
    let mut world = World::new(embed_params());
    let worker = ident(1);
    let verifier = ident(2);
    let result = hash(42);
    world.claim_and_accept(&worker, 1);
    world.submit(&worker, 1, result);
    world.accept_verifier(&verifier, 1);
    (world, worker, verifier, result)
}

#[test]
fn an_honest_tolerance_verdict_finalizes_the_task() {
    let (mut world, _worker, verifier, result) = accepted();
    let (model, runtime, cos_min) = pins();
    let attestation = verdict(&world, &verifier, result, model, runtime, cos_min + 500);
    world.emit(
        &verifier,
        EventBody::AttestTolerance {
            attestation: attestation.clone(),
        },
    );
    let (claim, submit) = world.refs(1);
    let certificate = ToleranceCertificate {
        task: TaskRef::new(world.job, 0),
        attempt: 1,
        result,
        approvals: vec![attestation],
    };
    world.emit(
        &ident(10),
        EventBody::FinalizeTolerance {
            fragment: 0,
            attempt: 1,
            claim,
            submit,
            verifiers: vec![offer_of(&world, &verifier)],
            certificate,
        },
    );
    assert_eq!(world.status(), TaskStatus::Finalized);
}

#[test]
fn a_verdict_with_another_model_is_refused() {
    let (mut world, _worker, verifier, result) = accepted();
    let (_model, runtime, cos_min) = pins();
    let other = verdict(&world, &verifier, result, hash(0xEE), runtime, cos_min);
    let error = world
        .try_emit(&verifier, EventBody::AttestTolerance { attestation: other })
        .unwrap_err();
    assert!(matches!(error, LedgerError::InvalidEvent(_)), "{error:?}");
}

#[test]
fn an_approval_under_the_threshold_is_refused() {
    let (mut world, _worker, verifier, result) = accepted();
    let (model, runtime, cos_min) = pins();
    let low = verdict(&world, &verifier, result, model, runtime, cos_min - 1);
    let error = world
        .try_emit(&verifier, EventBody::AttestTolerance { attestation: low })
        .unwrap_err();
    assert!(matches!(error, LedgerError::InvalidEvent(_)), "{error:?}");
}

#[test]
fn an_exact_attestation_is_refused_on_a_tolerance_workload() {
    let (mut world, _worker, verifier, result) = accepted();
    let exact = world.attestation(&verifier, 1, result, true);
    let error = world
        .try_emit(&verifier, EventBody::Attest { attestation: exact })
        .unwrap_err();
    assert!(matches!(error, LedgerError::InvalidEvent(_)), "{error:?}");
}
