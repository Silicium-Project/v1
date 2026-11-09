use super::support::*;
use crate::*;

fn completed(p: Params) -> (World, sil_types::Hash32, sil_crypto::SigningIdentity) {
    let worker = ident(60);
    let mut world = World::new(p);
    let result = hash(42);
    world.claim_and_accept(&worker, 1);
    world.submit(&worker, 1, result);
    (world, result, worker)
}

/// Phase 3 : les k = 2 premières offres sont retenues. Un saboteur qui propose en dernier n'est
/// pas retenu, donc son rejet ne compte pas.
#[test]
fn a_late_sybil_rejection_cannot_kill_a_result_that_needs_two_approvals() {
    let (mut world, result, _) = completed(Params {
        verifications: 2,
        ..Params::default()
    });
    let (griefer, honest_a, honest_b) = (ident(61), ident(62), ident(63));
    for verifier in [&honest_a, &honest_b, &griefer] {
        world.accept_verifier(verifier, 1);
    }
    world.attest(&griefer, 1, result, false);
    world.attest(&honest_a, 1, result, true);
    assert_eq!(
        world.status(),
        TaskStatus::Completed,
        "1 approbation < 2 requises"
    );
    world.attest(&honest_b, 1, result, true);
    assert_eq!(world.status(), TaskStatus::Verified);
}

#[test]
fn enough_rejections_reject_the_attempt_and_open_the_next() {
    let (mut world, result, _) = completed(Params::default());
    let verifier = ident(61);
    world.accept_verifier(&verifier, 1);
    world.attest(&verifier, 1, result, false);
    let task = world.task();
    assert_eq!(task.status, TaskStatus::Rejected);
    assert_eq!(task.next_attempt, Some(2));
}

/// Phase 3 : avec k = 1, seule la première offre est retenue. Un verdict d'une offre suivante est
/// ignoré, même contraire.
#[test]
fn a_verdict_from_an_unchosen_verifier_is_ignored() {
    let (mut world, result, _) = completed(Params::default());
    let (yes, no) = (ident(61), ident(62));
    world.accept_verifier(&yes, 1);
    world.accept_verifier(&no, 1);
    world.attest(&yes, 1, result, true);
    world.attest(&no, 1, result, false);
    assert_eq!(world.status(), TaskStatus::Verified);
    assert_eq!(world.task().attempts[0].verifiers.len(), 1);
}

#[test]
fn the_claimant_cannot_be_its_own_verifier() {
    let (mut world, result, worker) = completed(Params::default());
    world.accept_verifier(&worker, 1);
    assert!(world.task().attempts[0].verifiers.is_empty());
    world.attest(&worker, 1, result, true);
    assert_eq!(world.status(), TaskStatus::Completed);
}

#[test]
fn a_verdict_from_a_verifier_never_chosen_is_ignored() {
    let (mut world, result, _) = completed(Params::default());
    let stranger = ident(66);
    world.attest(&stranger, 1, result, true);
    assert_eq!(world.status(), TaskStatus::Completed);
}

#[test]
fn a_verdict_on_another_result_is_ignored() {
    let (mut world, _, _) = completed(Params::default());
    let verifier = ident(61);
    world.accept_verifier(&verifier, 1);
    world.attest(&verifier, 1, hash(77), true);
    assert_eq!(world.status(), TaskStatus::Completed);
}

#[test]
fn open_verifications_exclude_claimant_and_accepted_verifiers() {
    let (mut world, _, worker) = completed(Params {
        verifications: 2,
        ..Params::default()
    });
    let (v1, v2) = (ident(61), ident(62));
    world.emit(
        &worker,
        EventBody::VerifyOffer {
            fragment: 0,
            attempt: 1,
        },
    );
    world.emit(
        &v1,
        EventBody::VerifyOffer {
            fragment: 0,
            attempt: 1,
        },
    );
    world.emit(
        &v2,
        EventBody::VerifyOffer {
            fragment: 0,
            attempt: 1,
        },
    );
    // Phase 3 : v1 et v2 sont les deux premières offres : retenues, donc plus « ouvertes ».
    let verifiers: Vec<_> = world.task().attempts[0]
        .verifiers
        .iter()
        .map(|v| v.verifier)
        .collect();
    assert_eq!(verifiers, vec![v1.public(), v2.public()]);
    assert!(
        world
            .ledger
            .open_verifications(&world.job, 0, world.now)
            .is_empty()
    );
    assert!(!verifiers.contains(&worker.public()));
}

#[test]
fn approvals_returns_only_attestations_of_accepted_verifiers_that_approved() {
    let (mut world, result, _) = completed(Params {
        verifications: 2,
        ..Params::default()
    });
    let (yes, no, stranger) = (ident(61), ident(62), ident(66));
    world.accept_verifier(&yes, 1);
    world.accept_verifier(&no, 1);
    world.attest(&yes, 1, result, true);
    world.attest(&no, 1, result, false);
    world.attest(&stranger, 1, result, true);
    let approvals = world.ledger.approvals(&world.job, 0, 1, world.now);
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].verifier, yes.public());
    assert!(
        world
            .ledger
            .approvals(&world.job, 0, 2, world.now)
            .is_empty()
    );
    assert!(
        world
            .ledger
            .approvals(&world.job, 9, 1, world.now)
            .is_empty()
    );
}

#[test]
fn events_of_lists_an_authors_events_in_sequence_order() {
    let (world, _, worker) = completed(Params::default());
    let events = world.ledger.events_of(&world.job, &worker.public());
    assert_eq!(events.len(), 2, "Claim puis Submit");
    assert!(events[0].sequence < events[1].sequence);
}
