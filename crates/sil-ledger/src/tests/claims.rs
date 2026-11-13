//! Phase 2 (ADR 0005) : la tentative est retenue par le premier claim valide dans l'ordre total,
//! sans vote, et le bail se lit sur les heures des événements.

use super::support::*;
use crate::*;
use sil_quota::LedgerLimits;
use sil_types::{JobId, PublicKey};

fn claim(fragment: u32, attempt: u32) -> EventBody {
    EventBody::Claim { anchor_slot: 1, fragment, attempt }
}

/// Claimant de la tentative 1 vu par ce pair, à l'instant `now`.
fn holder(ledger: &Ledger, job: &JobId, now: u64) -> Option<PublicKey> {
    ledger
        .task_state(job, 0, now)?
        .attempts
        .first()
        .map(|attempt| attempt.claimant)
}

#[test]
fn a_claim_alone_retains_the_attempt() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    world.emit(&worker, claim(0, 1));
    let task = world.task();
    assert_eq!(task.status, TaskStatus::Claimed);
    assert_eq!(task.attempts[0].claimant, worker.public());
}

#[test]
fn a_later_claim_for_a_held_attempt_is_ignored() {
    let (first, second) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    world.emit(&first, claim(0, 1));
    world.emit(&second, claim(0, 1));
    assert_eq!(world.task().attempts.len(), 1);
    assert_eq!(world.task().attempts[0].claimant, first.public());
}

#[test]
fn the_earliest_valid_claim_wins_whatever_the_arrival_order() {
    let (early, late) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    world.now = 5_000;
    world.emit(&late, claim(0, 1)); // issued 5001
    world.now = 2_000;
    world.emit(&early, claim(0, 1)); // issued 2001, émis après mais daté avant
    let now = world.now;

    // Pair A : reçoit dans l'ordre du journal.
    let mut in_order = Ledger::open_to_all(LedgerLimits::default());
    assert!(feed(&mut in_order, &world.log).is_empty());
    // Pair B : reçoit le claim tardif en premier.
    let mut reversed = Ledger::open_to_all(LedgerLimits::default());
    let arrival = [
        world.log[0].clone(),
        world.log[2].clone(),
        world.log[1].clone(),
    ];
    assert!(feed(&mut reversed, &arrival).is_empty());

    assert_eq!(holder(&in_order, &world.job, now), Some(early.public()));
    assert_eq!(holder(&reversed, &world.job, now), Some(early.public()));
}

#[test]
fn a_claim_for_a_later_attempt_needs_the_previous_one() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    world.emit(&worker, claim(0, 2));
    assert!(world.task().attempts.is_empty());
}

#[test]
fn a_takeover_needs_the_previous_lease_to_be_over_by_the_claim_time() {
    let (w1, w2) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    world.emit(&w1, claim(0, 1)); // issued 1001, bail jusqu'à 61001
    world.now = 30_000;
    world.emit(&w2, claim(0, 2)); // issued 30001 : bail encore actif
    assert_eq!(world.task().attempts.len(), 1);
    world.now = 61_500;
    world.emit(&w2, claim(0, 2)); // issued 61501 : bail échu
    let task = world.task();
    assert_eq!(task.attempts.len(), 2);
    assert_eq!(task.attempts[1].claimant, w2.public());
}

#[test]
fn a_renewal_extends_the_lease_only_when_it_is_issued_before_expiry() {
    let worker = ident(60);
    let mut world = World::new(Params::default());
    world.emit(&worker, claim(0, 1)); // bail jusqu'à 61001
    world.now = 50_000;
    world.emit(
        &worker,
        EventBody::Renew {
            fragment: 0,
            attempt: 1,
        },
    ); // émis avant échéance : bail jusqu'à 121001
    world.now = 130_000;
    world.emit(
        &worker,
        EventBody::Renew {
            fragment: 0,
            attempt: 1,
        },
    ); // émis après : sans effet
    let attempt = &world.task().attempts[0];
    assert_eq!(attempt.renewals, 1);
    assert_eq!(attempt.lease_expires_at_ms, 121_001);
}

/// Phase 3 : les k premières offres valides dans l'ordre total sont les vérificateurs, quel que
/// soit l'ordre d'arrivée.
#[test]
fn the_first_offers_in_total_order_are_the_verifiers() {
    let (worker, late, early) = (ident(60), ident(61), ident(62));
    let mut world = World::new(Params::default());
    world.claim_and_accept(&worker, 1);
    let result = hash(42);
    world.submit(&worker, 1, result);
    let now = world.now;
    world.now = 5_000;
    world.emit(
        &late,
        EventBody::VerifyOffer {
            fragment: 0,
            attempt: 1,
        },
    ); // daté 5001, émis en premier
    world.now = 2_000;
    world.emit(
        &early,
        EventBody::VerifyOffer {
            fragment: 0,
            attempt: 1,
        },
    ); // daté 2001, émis après
    let verifier_of = |ledger: &Ledger| {
        ledger.task_state(&world.job, 0, now).unwrap().attempts[0]
            .verifiers
            .iter()
            .map(|v| v.verifier)
            .collect::<Vec<_>>()
    };
    assert_eq!(verifier_of(&world.ledger), vec![early.public()]);
    // Un pair qui reçoit le claim tardif en premier obtient le même choix.
    let mut reversed = Ledger::open_to_all(LedgerLimits::default());
    let mut arrival = world.log.clone();
    let last = arrival.len() - 1;
    arrival.swap(last - 1, last);
    assert!(feed(&mut reversed, &arrival).is_empty());
    assert_eq!(verifier_of(&reversed), vec![early.public()]);
}

/// ADR 0006 : pas de liste blanche. Une offre valide d'un auteur hors comité est retenue dans
/// l'ordre total, comme les autres ; la réputation on-chain est contrôlée au règlement.
#[test]
fn an_offer_from_outside_the_committee_is_chosen_in_time_order() {
    let (worker, member, outsider) = (ident(60), ident(61), ident(66));
    let mut world = World::new(Params::default());
    world.claim_and_accept(&worker, 1);
    world.submit(&worker, 1, hash(42));
    world.now = 1_100;
    world.emit(&outsider, offer_body()); // daté 1101, premier
    world.now = 1_200;
    world.emit(&member, offer_body()); // daté 1201
    let chosen: Vec<_> = world.task().attempts[0]
        .verifiers
        .iter()
        .map(|v| v.verifier)
        .collect();
    assert_eq!(chosen, vec![outsider.public()]);
}

/// Phase 4 : un `Finalize` ne lit que les offres qu'il désigne. Une offre plus ancienne, reçue
/// après, qui évince le vérificateur approbateur ne défait pas la finalisation.
#[test]
fn a_late_earlier_offer_cannot_undo_a_finalization() {
    let (worker, approver, latecomer) = (ident(60), ident(61), ident(62));
    let mut world = World::new(Params::default());
    world.claim_and_accept(&worker, 1);
    let result = hash(42);
    world.submit(&worker, 1, result);
    world.accept_verifier(&approver, 1);
    world.attest(&approver, 1, result, true);
    let certificate = world.certificate(&[&approver], 1, result);
    world.finalize(1, 1, &certificate);
    assert_eq!(world.status(), TaskStatus::Finalized);
    world.now = 1_100;
    world.emit(&latecomer, offer_body()); // daté 1101 : avant l'offre de l'approbateur
    assert_eq!(
        world.status(),
        TaskStatus::Finalized,
        "la finalisation tient"
    );
}

fn offer_body() -> EventBody {
    EventBody::VerifyOffer {
        fragment: 0,
        attempt: 1,
    }
}

/// Un résultat émis après le bail de son claim ne peut pas être certifié, même désigné.
#[test]
fn a_result_submitted_after_the_lease_cannot_be_certified() {
    let (worker, verifier) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    let claim_event = world.emit(&worker, claim(0, 1)); // bail jusqu'à 61001
    let result = hash(42);
    world.now = 70_000;
    let submit = world.emit(
        &worker,
        EventBody::Submit {
            fragment: 0,
            attempt: 1,
            result,
            size: 32,
            milestones_root: hash(9),
        },
    );
    world.accept_verifier(&verifier, 1);
    world.attest(&verifier, 1, result, true);
    let certificate = world.certificate(&[&verifier], 1, result);
    let finalize = EventBody::Finalize {
        fragment: 0,
        attempt: 1,
        claim: claim_event.id().unwrap(),
        submit: submit.id().unwrap(),
        verifiers: Vec::new(),
        certificate,
    };
    world.emit(&ident(10), finalize);
    assert_eq!(world.status(), TaskStatus::LeaseExpired);
    assert!(world.task().finalized.is_none());
}

/// Régression de la phase 2, corrigée en phase 3 : un claim daté avant celui qui a abouti, reçu
/// après la finalisation, change le vainqueur de la tentative 1 sans défaire la certification.
#[test]
fn a_backdated_claim_cannot_undo_a_finalization() {
    let (worker, verifier, thief) = (ident(60), ident(61), ident(62));
    let mut world = World::new(Params::default());
    let result = world.run_to_verified(&worker, &verifier);
    let certificate = world.certificate(&[&verifier], 1, result);
    world.finalize(2, 1, &certificate);
    assert_eq!(world.status(), TaskStatus::Finalized);
    world.now = 500;
    world.emit(&thief, claim(0, 1));
    assert_eq!(world.status(), TaskStatus::Finalized);
}

#[test]
fn the_first_valid_claim_wins_the_attempt_without_any_vote() {
    let (first, second) = (ident(60), ident(61));
    let mut world = World::new(Params::default());
    world.emit(&first, claim(0, 1));
    world.emit(&second, claim(0, 1));
    assert_eq!(world.task().attempts[0].claimant, first.public());
}
