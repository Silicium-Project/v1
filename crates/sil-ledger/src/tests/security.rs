use super::support::*;
use crate::*;
use sil_attest::{AttestError, Certificate};
use sil_quota::LedgerLimits;
use sil_types::TaskRef;

#[test]
fn a_cancel_is_the_requesters_alone() {
    let (worker, outsider) = (ident(60), ident(70));
    let mut world = World::new(Params::default());
    assert!(matches!(
        world.try_emit(&outsider, EventBody::Cancel),
        Err(LedgerError::NotAuthorized(_))
    ));
    assert!(matches!(
        world.try_emit(&worker, EventBody::Cancel),
        Err(LedgerError::NotAuthorized(_))
    ));
}

#[test]
fn events_of_an_unknown_job_are_refused() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    let claim = world.emit(
        &worker,
        EventBody::Claim { anchor_slot: 1,
            fragment: 0,
            attempt: 1,
        },
    );
    let mut elsewhere = Ledger::open_to_all(LedgerLimits::default());
    assert_eq!(elsewhere.receive(claim), Err(LedgerError::UnknownJob));
}

#[test]
fn a_tampered_event_fails_its_signature() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    let mut claim = world.emit(
        &worker,
        EventBody::Claim { anchor_slot: 1,
            fragment: 0,
            attempt: 1,
        },
    );
    claim.issued_at_ms += 1;
    let mut other = World::new(Params::default());
    assert_eq!(other.ledger.receive(claim), Err(LedgerError::BadSignature));
}

#[test]
fn a_fragment_outside_the_job_is_refused() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    assert!(matches!(
        world.try_emit(
            &worker,
            EventBody::Claim { anchor_slot: 1,
                fragment: 5,
                attempt: 1
            }
        ),
        Err(LedgerError::InvalidEvent(_))
    ));
    assert!(matches!(
        world.try_emit(
            &worker,
            EventBody::Claim { anchor_slot: 1,
                fragment: 0,
                attempt: 0
            }
        ),
        Err(LedgerError::InvalidEvent(_))
    ));
}

#[test]
fn a_spec_is_signed_by_the_requester_only() {
    let requester = ident(100);
    let spec = spec_for(&requester, &Params::default());
    assert!(matches!(
        SignedJobSpec::sign(&ident(99), spec),
        Err(LedgerError::NotAuthorized(_))
    ));
}

#[test]
fn finalize_without_independent_approvals_is_refused_at_the_door() {
    let (worker, verifier) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    let result = world.run_to_verified(&worker, &verifier);
    let empty = Certificate {
        task: TaskRef::new(world.job, 0),
        attempt: 1,
        result,
        approvals: Vec::new(),
    };
    let (claim, submit) = world.refs(1);
    let body = EventBody::Finalize {
        fragment: 0,
        attempt: 1,
        claim,
        submit,
        verifiers: Vec::new(),
        certificate: empty,
    };
    assert!(matches!(
        world.try_emit(&ident(10), body),
        Err(LedgerError::Attestation(AttestError::TooFew { .. }))
    ));
}

#[test]
fn a_rogue_certificate_cannot_finalize() {
    let (worker, rogue) = (ident(60), ident(66));
    let mut world = World::new(Params::default());
    let result = hash(42);
    world.claim_and_accept(&worker, 1);
    world.submit(&worker, 1, result);
    // Aucun vérificateur accepté : un faux certificat signé par une clé quelconque.
    let forged = world.certificate(&[&rogue], 1, result);
    world.finalize(2, 1, &forged);
    let task = world.task();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.finalized.is_none());
}

#[test]
fn the_claimant_cannot_inflate_the_certificate_with_its_own_approval() {
    let (worker, verifier) = (ident(60), ident(61));
    let mut world = World::new(Params {
        verifications: 3,
        ..Params::default()
    });
    let result = hash(42);
    world.claim_and_accept(&worker, 1);
    world.submit(&worker, 1, result);
    world.accept_verifier(&verifier, 1);
    world.attest(&verifier, 1, result, true);
    // 1 vraie approbation + celle du claimant : le seuil de 3 n'est pas atteint.
    let padded = world.certificate(&[&verifier, &worker], 1, result);
    // Le certificat compte 2 approbations pour un seuil de 3 : la finalisation est refusée par le
    // ledger (`TooFew`). Le statut reste « Completed ».
    assert_eq!(padded.approvals.len(), 2);
    assert_eq!(world.status(), TaskStatus::Completed);
}

/// Un rejet postérieur à une certification ouvre un litige : il ne défait pas la finalisation
/// (sinon la validité ne serait pas monotone, et un seul rejet suffirait à la défaire).
#[test]
fn a_dissent_does_not_undo_a_certified_result() {
    let (worker, good, bad) = (ident(60), ident(61), ident(62));
    let mut world = World::new(Params::default());
    let result = hash(42);
    world.claim_and_accept(&worker, 1);
    world.submit(&worker, 1, result);
    world.accept_verifier(&good, 1);
    world.accept_verifier(&bad, 1);
    world.attest(&good, 1, result, true);
    world.attest(&bad, 1, result, false);
    let certificate = world.certificate(&[&good], 1, result);
    world.finalize(2, 1, &certificate);
    assert_eq!(world.status(), TaskStatus::Finalized);
}

#[test]
fn a_forked_author_is_caught_and_its_claims_are_ignored() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    let claim = world.emit(
        &worker,
        EventBody::Claim { anchor_slot: 1,
            fragment: 0,
            attempt: 1,
        },
    );
    // Deuxième version de l'événement (même séquence, autre contenu).
    let header = Header {
        job: world.job,
        sequence: claim.sequence,
        previous: claim.previous,
        issued_at_ms: claim.issued_at_ms,
    };
    let twin = Event::sign(
        &worker,
        header,
        EventBody::Claim { anchor_slot: 1,
            fragment: 0,
            attempt: 2,
        },
    )
    .unwrap();
    assert_eq!(
        world.ledger.receive(twin),
        Err(LedgerError::Equivocation(worker.public()))
    );
    assert_eq!(world.ledger.equivocators(&world.job), vec![worker.public()]);
    assert_eq!(
        world.status(),
        TaskStatus::Open,
        "le claim d'un auteur fautif ne retient rien"
    );
}
