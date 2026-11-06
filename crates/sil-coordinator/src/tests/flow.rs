use super::support::*;
use crate::Coordinator;
use sil_ledger::TaskStatus;
use sil_reputation::Outcome;
use sil_types::{JobId, PublicKey};

#[test]
fn the_committee_alone_drives_a_job_from_claim_to_finalization() {
    let (worker, verifier) = (ident(60), ident(70));
    let mut world = World::new();
    world.claim(&worker, 1);
    world.offer(&verifier, 1);
    let mut submitted = false;
    let mut attested = false;
    for _ in 0..12 {
        world.drive();
        let task = world.task();
        match task.status {
            TaskStatus::Claimed if !submitted => {
                world.submit(&worker, 1);
                submitted = true;
            }
            TaskStatus::Completed
                if !attested
                    && task.attempts[0]
                        .verifiers
                        .iter()
                        .any(|v| v.verdict.is_none()) =>
            {
                world.attest(&verifier, 1, true);
                attested = true;
            }
            TaskStatus::Finalized => break,
            _ => {}
        }
    }
    let task = world.task();
    assert_eq!(task.status, TaskStatus::Finalized, "{task:#?}");
    let finalized = task.finalized.unwrap();
    assert_eq!(finalized.claimant, worker.public());
    assert_eq!(finalized.approvers, vec![verifier.public()]);
}

#[test]
fn deciding_twice_without_new_events_emits_nothing_the_second_time() {
    let worker = ident(60);
    let mut world = World::new();
    world.claim(&worker, 1);
    assert!(world.drive() > 0);
    assert_eq!(world.drive(), 0, "aucun vote en double");
    assert_eq!(world.drive(), 0);
}

#[test]
fn the_best_reputation_wins_the_claim() {
    let (newcomer, veteran) = (ident(60), ident(61));
    let mut world = World::new();
    for _ in 0..5 {
        let insiders: Vec<PublicKey> = world.coordinators.iter().map(Coordinator::public).collect();
        world.reputation.observe(
            &ident(100).public(),
            &veteran.public(),
            &insiders,
            Outcome::Completed { reward: 10 },
            world.now,
        );
    }
    world.claim(&newcomer, 1); // arrivé en premier
    world.claim(&veteran, 1);
    world.drive();
    assert_eq!(world.task().attempts[0].claimant, veteran.public());
}

#[test]
fn without_independent_approval_nothing_is_finalized() {
    let (worker, verifier) = (ident(60), ident(70));
    let mut world = World::new();
    world.claim(&worker, 1);
    world.offer(&verifier, 1);
    world.drive();
    world.submit(&worker, 1);
    for _ in 0..4 {
        world.drive();
    }
    let task = world.task();
    assert_eq!(
        task.status,
        TaskStatus::Completed,
        "verificateur retenu mais muet"
    );
    assert!(task.finalized.is_none());
    assert_eq!(task.attempts[0].verifiers.len(), 1);
}

#[test]
fn a_rejected_attempt_goes_to_the_next_claimant() {
    let (first, second, verifier) = (ident(60), ident(61), ident(70));
    let mut world = World::new();
    world.claim(&first, 1);
    world.claim(&second, 2);
    world.offer(&verifier, 1);
    world.drive();
    world.submit(&first, 1);
    world.drive();
    world.attest(&verifier, 1, false);
    for _ in 0..4 {
        world.drive();
    }
    let task = world.task();
    assert_eq!(task.attempts.len(), 2, "{task:#?}");
    assert_eq!(task.attempts[1].claimant, second.public());
    assert_eq!(task.status, TaskStatus::Claimed);
}

#[test]
fn only_committee_members_decide_and_unknown_jobs_are_ignored() {
    let world = World::new();
    let outsider = Coordinator::new(ident(99));
    let worker = ident(60);
    let mut other = World::new();
    other.claim(&worker, 1);
    assert!(
        outsider
            .decide(&other.ledger, &other.reputation, &other.job, other.now)
            .is_empty()
    );
    let unknown = JobId::from_bytes([5; 32]);
    assert!(
        world.coordinators[0]
            .decide(&world.ledger, &world.reputation, &unknown, world.now)
            .is_empty()
    );
}
