use super::support::*;
use crate::*;
use sil_attest::{AttestError, Certificate};
use sil_quota::LedgerLimits;
use sil_types::TaskRef;

#[test]
fn only_committee_members_can_vote() {
    let (worker, outsider) = (ident(60), ident(70));
    let mut world = World::new(Params::default());
    let claim = world.emit(
        &worker,
        EventBody::Claim {
            fragment: 0,
            attempt: 1,
        },
    );
    let vote = EventBody::Accept {
        fragment: 0,
        attempt: 1,
        claim: claim.id().unwrap(),
    };
    assert!(matches!(
        world.try_emit(&outsider, vote.clone()),
        Err(LedgerError::NotAuthorized(_))
    ));
    assert!(matches!(
        world.try_emit(&worker, vote),
        Err(LedgerError::NotAuthorized(_))
    ));
}

#[test]
fn events_of_an_unknown_job_are_refused() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    let claim = world.emit(
        &worker,
        EventBody::Claim {
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
        EventBody::Claim {
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
            EventBody::Claim {
                fragment: 5,
                attempt: 1
            }
        ),
        Err(LedgerError::InvalidEvent(_))
    ));
    assert!(matches!(
        world.try_emit(
            &worker,
            EventBody::Claim {
                fragment: 0,
                attempt: 0
            }
        ),
        Err(LedgerError::InvalidEvent(_))
    ));
}

#[test]
fn a_spec_needs_a_strict_majority_committee_and_the_requester_signature() {
    let requester = ident(100);
    let committee = vec![ident(10), ident(11), ident(12), ident(13)];
    let weak = Params {
        committee: 4,
        threshold: 2,
        ..Params::default()
    };
    let spec = spec_for(&requester, &committee, &weak);
    assert!(matches!(
        SignedJobSpec::sign(&requester, spec.clone()),
        Err(LedgerError::InvalidSpec(_))
    ));
    let strong = Params {
        committee: 4,
        threshold: 3,
        ..Params::default()
    };
    let spec = spec_for(&requester, &committee, &strong);
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
    let body = EventBody::Finalize {
        fragment: 0,
        attempt: 1,
        certificate: empty,
    };
    assert!(matches!(
        world.try_emit(&ident(10), body),
        Err(LedgerError::Attestation(AttestError::TooFew { .. }))
    ));
}

#[test]
fn a_colluding_committee_cannot_finalize_with_a_rogue_certificate() {
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
        verifications: 2,
        ..Params::default()
    });
    let result = hash(42);
    world.claim_and_accept(&worker, 1);
    world.submit(&worker, 1, result);
    world.accept_verifier(&verifier, 1);
    world.attest(&verifier, 1, result, true);
    // 1 vraie approbation + celle du claimant : le seuil de 2 n'est pas atteint.
    let padded = world.certificate(&[&verifier, &worker], 1, result);
    world.finalize(2, 1, &padded);
    assert_eq!(world.status(), TaskStatus::Completed);
}

#[test]
fn a_rejected_attempt_cannot_be_finalized() {
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
    assert_eq!(world.status(), TaskStatus::Disputed, "pas Finalized");
}

#[test]
fn a_forked_author_is_caught_and_its_votes_are_ignored() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    let claim = world.emit(
        &worker,
        EventBody::Claim {
            fragment: 0,
            attempt: 1,
        },
    );
    let claim_id = claim.id().unwrap();
    let member = ident(10);
    let vote = world.emit(
        &member,
        EventBody::Accept {
            fragment: 0,
            attempt: 1,
            claim: claim_id,
        },
    );
    // Deuxième version de l'événement (même séquence, autre contenu).
    let header = Header {
        job: world.job,
        sequence: vote.sequence,
        previous: vote.previous,
        issued_at_ms: vote.issued_at_ms,
    };
    let twin = Event::sign(
        &member,
        header,
        EventBody::Accept {
            fragment: 0,
            attempt: 1,
            claim: hash(0xee),
        },
    )
    .unwrap();
    assert_eq!(
        world.ledger.receive(twin),
        Err(LedgerError::Equivocation(member.public()))
    );
    assert_eq!(world.ledger.equivocators(&world.job), vec![member.public()]);
    // Le second vote (membre 11) seul ne suffit plus : le vote du fautif est ignoré.
    world.emit(
        &ident(11),
        EventBody::Accept {
            fragment: 0,
            attempt: 1,
            claim: claim_id,
        },
    );
    assert_eq!(world.status(), TaskStatus::Open);
}
