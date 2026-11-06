use super::support::*;
use crate::*;

#[test]
fn full_lifecycle_needs_committee_votes_and_a_certificate() {
    let (worker, verifier) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    assert_eq!(world.status(), TaskStatus::Open);
    assert_eq!(world.task().next_attempt, Some(1));

    let claim = world.emit(
        &worker,
        EventBody::Claim {
            fragment: 0,
            attempt: 1,
        },
    );
    assert_eq!(world.ledger.open_claims(&world.job, 0, world.now).len(), 1);
    let accept = EventBody::Accept {
        fragment: 0,
        attempt: 1,
        claim: claim.id().unwrap(),
    };
    world.vote(1, &accept);
    assert_eq!(world.status(), TaskStatus::Open, "1 vote < seuil 2");
    world.vote(2, &accept);
    assert_eq!(world.status(), TaskStatus::Claimed);
    assert!(
        world
            .ledger
            .open_claims(&world.job, 0, world.now)
            .is_empty()
    );

    let result = hash(42);
    world.submit(&worker, 1, result);
    assert_eq!(world.status(), TaskStatus::Completed);
    assert_eq!(
        world
            .ledger
            .verification_slots_open(&world.job, 0, world.now),
        1
    );

    world.accept_verifier(&verifier, 1);
    assert_eq!(
        world
            .ledger
            .verification_slots_open(&world.job, 0, world.now),
        0
    );
    world.attest(&verifier, 1, result, true);
    assert_eq!(world.status(), TaskStatus::Verified);

    let certificate = world.certificate(&[&verifier], 1, result);
    world.finalize(1, 1, &certificate);
    assert_eq!(
        world.status(),
        TaskStatus::Verified,
        "1 vote de finalisation < seuil"
    );
    world.finalize(2, 1, &certificate);
    let task = world.task();
    assert_eq!(task.status, TaskStatus::Finalized);
    let finalized = task.finalized.unwrap();
    assert_eq!(finalized.claimant, worker.public());
    assert_eq!(finalized.approvers, vec![verifier.public()]);
    let job = world.ledger.job_state(&world.job, world.now).unwrap();
    assert_eq!(job.status, JobStatus::Finalized);
}

#[test]
fn expired_lease_opens_the_next_attempt_to_other_workers() {
    let (w1, w2) = (ident(60), ident(62));
    let mut world = World::new(Params::default());
    world.claim_and_accept(&w1, 1);
    assert_eq!(world.status(), TaskStatus::Claimed);
    world.now += 100_000;
    let task = world.task();
    assert_eq!(task.status, TaskStatus::LeaseExpired);
    assert_eq!(task.next_attempt, Some(2));

    world.emit(
        &w1,
        EventBody::Claim {
            fragment: 0,
            attempt: 2,
        },
    );
    world.emit(
        &w2,
        EventBody::Claim {
            fragment: 0,
            attempt: 2,
        },
    );
    let offers = world.ledger.open_claims(&world.job, 0, world.now);
    assert_eq!(offers.len(), 1, "l'ancien claimant est exclu");
    assert_eq!(offers[0].author, w2.public());

    world.claim_and_accept(&w2, 2);
    assert_eq!(world.task().attempts.len(), 2);
    assert_eq!(world.status(), TaskStatus::Claimed);
}

#[test]
fn renewals_extend_the_lease_and_release_ends_it() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    world.claim_and_accept(&worker, 1);
    let before = world.task().attempts[0].lease_expires_at_ms;
    world.emit(
        &worker,
        EventBody::Renew {
            fragment: 0,
            attempt: 1,
        },
    );
    assert_eq!(
        world.task().attempts[0].lease_expires_at_ms,
        before + 60_000
    );
    world.emit(
        &worker,
        EventBody::Release {
            fragment: 0,
            attempt: 1,
        },
    );
    assert_eq!(world.status(), TaskStatus::Released);
    assert_eq!(world.task().next_attempt, Some(2));
}

#[test]
fn a_late_result_still_counts_until_a_new_attempt_is_accepted() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    world.claim_and_accept(&worker, 1);
    world.now += 100_000;
    assert_eq!(world.status(), TaskStatus::LeaseExpired);
    world.submit(&worker, 1, hash(5));
    assert_eq!(world.status(), TaskStatus::Completed);
}

#[test]
fn last_failed_attempt_fails_the_task() {
    let worker = ident(60);
    let mut world = World::new(Params {
        max_attempts: 1,
        ..Params::default()
    });
    world.claim_and_accept(&worker, 1);
    world.now += 100_000;
    let task = world.task();
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.next_attempt, None);
}

#[test]
fn only_the_requester_cancels() {
    let mut world = World::new(Params::default());
    let outsider = ident(70);
    assert!(matches!(
        world.try_emit(&outsider, EventBody::Cancel),
        Err(LedgerError::NotAuthorized(_))
    ));
    let requester = ident(100);
    world.emit(&requester, EventBody::Cancel);
    assert_eq!(world.status(), TaskStatus::Cancelled);
    let job = world.ledger.job_state(&world.job, world.now).unwrap();
    assert_eq!(job.status, JobStatus::Cancelled);
}

#[test]
fn deadline_expires_the_job() {
    let mut world = World::new(Params {
        expires_at_ms: 2_000,
        ..Params::default()
    });
    assert_eq!(world.status(), TaskStatus::Open);
    world.now = 5_000;
    assert_eq!(world.status(), TaskStatus::Expired);
    let job = world.ledger.job_state(&world.job, world.now).unwrap();
    assert_eq!(job.status, JobStatus::Expired);
}
