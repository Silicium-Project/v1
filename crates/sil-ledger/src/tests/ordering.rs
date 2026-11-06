use super::support::*;
use crate::*;
use sil_quota::LedgerLimits;

fn full_flow() -> World {
    let (worker, verifier) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    let result = world.run_to_verified(&worker, &verifier);
    let certificate = world.certificate(&[&verifier], 1, result);
    world.finalize(2, 1, &certificate);
    world
}

fn replica(world: &World, events: &[Event]) -> Ledger {
    let mut ledger = Ledger::open_to_all(LedgerLimits::default());
    // La spec voyage avec le premier événement ; le reste peut arriver dans le désordre.
    ledger.receive(world.log[0].clone()).unwrap();
    feed(&mut ledger, events);
    ledger
}

#[test]
fn replicas_converge_whatever_the_arrival_order() {
    let world = full_flow();
    let expected = world.ledger.job_state(&world.job, world.now).unwrap();
    assert_eq!(expected.status, JobStatus::Finalized);

    let mut reversed: Vec<Event> = world.log[1..].to_vec();
    reversed.reverse();
    let mut rotated: Vec<Event> = world.log[1..].to_vec();
    rotated.rotate_left(5);

    for events in [reversed, rotated] {
        let copy = replica(&world, &events);
        assert_eq!(copy.job_state(&world.job, world.now).unwrap(), expected);
        assert!(copy.gaps().is_empty());
    }
}

#[test]
fn duplicates_change_nothing() {
    let world = full_flow();
    let mut copy = replica(&world, &world.log[1..]);
    let before = copy.job_state(&world.job, world.now).unwrap();
    let errors = feed(&mut copy, &world.log);
    assert!(errors.is_empty());
    assert_eq!(copy.job_state(&world.job, world.now).unwrap(), before);
}

#[test]
fn a_gap_is_reported_and_filled_from_a_peer() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    for attempt in 1..=3 {
        world.emit(
            &worker,
            EventBody::Claim {
                fragment: 0,
                attempt,
            },
        );
    }
    let mut copy = Ledger::open_to_all(LedgerLimits::default());
    copy.receive(world.log[0].clone()).unwrap();
    let last = world.log.last().unwrap().clone();
    let receipt = copy.receive(last).unwrap();
    assert!(receipt.pending && receipt.accepted.is_empty());
    let gaps = copy.gaps();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].author, worker.public());
    assert_eq!(gaps[0].after_sequence, 0);

    let missing =
        world
            .ledger
            .events_after(&world.job, &worker.public(), gaps[0].after_sequence, 64);
    assert_eq!(missing.len(), 3);
    feed(&mut copy, &missing);
    assert!(copy.gaps().is_empty());
    assert_eq!(copy.heads(&world.job).get(&worker.public()), Some(&3));
}

#[test]
fn a_late_predecessor_releases_the_buffered_successors() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    for attempt in 1..=3 {
        world.emit(
            &worker,
            EventBody::Claim {
                fragment: 0,
                attempt,
            },
        );
    }
    let mut copy = Ledger::open_to_all(LedgerLimits::default());
    copy.receive(world.log[0].clone()).unwrap();
    copy.receive(world.log[3].clone()).unwrap();
    copy.receive(world.log[2].clone()).unwrap();
    let receipt = copy.receive(world.log[1].clone()).unwrap();
    assert_eq!(
        receipt.accepted.len(),
        3,
        "l'événement et ses deux suivants"
    );
}

#[test]
fn events_since_walks_the_journal_in_acceptance_order_with_a_cursor() {
    let world = full_flow();
    let total = world.log.len();
    let first = world.ledger.events_since(&world.job, 0, 5);
    assert_eq!(first.len(), 5);
    assert!(
        matches!(first[0].body, EventBody::JobPublished { .. }),
        "la spec d'abord"
    );
    let rest = world.ledger.events_since(&world.job, 5, 1000);
    assert_eq!(first.len() + rest.len(), total);
    assert!(world.ledger.events_since(&world.job, total, 10).is_empty());

    // Un retardataire qui rejoue ce flux dans l'ordre reconstruit le même état.
    let mut copy = Ledger::open_to_all(LedgerLimits::default());
    let mut cursor = 0;
    loop {
        let batch = world.ledger.events_since(&world.job, cursor, 4);
        if batch.is_empty() {
            break;
        }
        cursor += batch.len();
        assert!(feed(&mut copy, &batch).is_empty());
    }
    assert_eq!(
        copy.job_state(&world.job, world.now),
        world.ledger.job_state(&world.job, world.now)
    );
}
