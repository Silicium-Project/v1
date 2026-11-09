use super::support::*;
use crate::*;
use sil_quota::LedgerLimits;

fn limits(change: impl FnOnce(&mut LedgerLimits)) -> LedgerLimits {
    let mut limits = LedgerLimits::default();
    change(&mut limits);
    limits
}

/// Événements de `worker` : n claims successifs, dans un job publié par `world`.
fn claims(world: &mut World, worker: &sil_crypto::SigningIdentity, n: u32) -> Vec<Event> {
    (1..=n)
        .map(|attempt| {
            world.emit(
                worker,
                EventBody::Claim {
                    fragment: 0,
                    attempt,
                },
            )
        })
        .collect()
}

#[test]
fn an_author_cannot_fill_a_job() {
    let worker = ident(60);
    let mut world = World::with_limits(
        Params::default(),
        limits(|l| l.max_events_per_author_job = 3),
    );
    claims(&mut world, &worker, 3);
    assert_eq!(
        world.try_emit(
            &worker,
            EventBody::Claim {
                fragment: 0,
                attempt: 4
            }
        ),
        Err(LedgerError::AuthorQuota)
    );
}

#[test]
fn a_job_has_an_event_ceiling() {
    let mut world = World::with_limits(Params::default(), limits(|l| l.max_events_per_job = 3));
    claims(&mut world, &ident(60), 2);
    assert_eq!(
        world.try_emit(
            &ident(61),
            EventBody::Claim {
                fragment: 0,
                attempt: 1
            }
        ),
        Err(LedgerError::JobFull)
    );
}

#[test]
fn a_job_has_a_byte_ceiling() {
    let mut world = World::with_limits(Params::default(), limits(|l| l.max_bytes_per_job = 1500));
    let mut refused = false;
    for attempt in 1..=10 {
        let who = ident(60 + attempt as u8);
        refused |= world
            .try_emit(
                &who,
                EventBody::Claim {
                    fragment: 0,
                    attempt: 1,
                },
            )
            .is_err_and(|e| e == LedgerError::JobBytesFull);
    }
    assert!(refused);
}

#[test]
fn oversized_events_are_refused() {
    let mut ledger = Ledger::open_to_all(limits(|l| l.max_event_bytes = 100));
    let requester = ident(100);
    let spec = spec_for(&requester, &Params::default());
    assert_eq!(
        ledger.publish_job(&requester, spec, 1).unwrap_err(),
        LedgerError::EventTooLarge
    );
}

#[test]
fn job_count_is_capped_globally_and_per_requester() {
    let mut ledger = Ledger::open_to_all(limits(|l| {
        l.max_jobs = 2;
        l.max_jobs_per_requester = 1;
    }));
    let p = Params::default();
    let make = |requester: u8, nonce: u64| {
        let who = ident(requester);
        let mut spec = spec_for(&who, &p);
        spec.nonce = nonce;
        (who, spec)
    };
    let (a, spec_a) = make(100, 1);
    ledger.publish_job(&a, spec_a, 1).unwrap();
    let (_, again) = make(100, 2);
    assert_eq!(
        ledger.publish_job(&a, again, 2).unwrap_err(),
        LedgerError::TooManyJobsForRequester
    );
    let (b, spec_b) = make(101, 1);
    ledger.publish_job(&b, spec_b, 3).unwrap();
    let (c, spec_c) = make(102, 1);
    assert_eq!(
        ledger.publish_job(&c, spec_c, 4).unwrap_err(),
        LedgerError::TooManyJobs
    );
}

#[test]
fn the_waiting_buffer_and_the_sequence_gap_are_bounded() {
    let worker = ident(60);
    let mut source = World::new(Params::default());
    let events = claims(&mut source, &worker, 6);

    let mut small = Ledger::open_to_all(limits(|l| l.max_pending_per_author_job = 2));
    small.receive(source.log[0].clone()).unwrap();
    assert!(small.receive(events[2].clone()).unwrap().pending);
    assert!(small.receive(events[3].clone()).unwrap().pending);
    assert_eq!(
        small.receive(events[4].clone()),
        Err(LedgerError::PendingFull)
    );

    let mut strict = Ledger::open_to_all(limits(|l| l.max_sequence_gap = 1));
    strict.receive(source.log[0].clone()).unwrap();
    assert_eq!(
        strict.receive(events[2].clone()),
        Err(LedgerError::SequenceTooFar)
    );
}
