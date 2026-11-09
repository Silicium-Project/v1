use crate::{Outcome, Reputation, ReputationConfig, verdict_outcome};
use sil_types::PublicKey;

#[test]
fn a_machine_is_a_newcomer_until_it_has_a_record() {
    let mut r = reputation();
    assert!(r.is_newcomer(&key(50), 0));
    for _ in 0..2 {
        r.observe(
            &key(1),
            &key(50),
            &[key(10)],
            Outcome::Completed { reward: 0 },
            0,
        );
    }
    assert!(r.is_newcomer(&key(50), 0));
    r.observe(
        &key(1),
        &key(50),
        &[key(10)],
        Outcome::Completed { reward: 0 },
        0,
    );
    assert!(!r.is_newcomer(&key(50), 0));
}

#[test]
fn a_verifier_is_right_when_its_verdict_matches_the_decision() {
    assert_eq!(
        verdict_outcome(true, true),
        Outcome::Completed { reward: 0 }
    );
    assert_eq!(
        verdict_outcome(false, false),
        Outcome::Completed { reward: 0 }
    );
    assert_eq!(verdict_outcome(true, false), Outcome::Failed);
    assert_eq!(verdict_outcome(false, true), Outcome::Failed);
}

fn key(n: u8) -> PublicKey {
    PublicKey::from_bytes([n; 32])
}

const DAY: u64 = 24 * 3600 * 1000;

fn reputation() -> Reputation {
    Reputation::new(ReputationConfig::default(), [key(1)])
}

fn done(r: &mut Reputation, who: u8, now: u64) -> bool {
    r.observe(
        &key(1),
        &key(who),
        &[key(10)],
        Outcome::Completed { reward: 10 },
        now,
    )
}

#[test]
fn newcomers_start_neutral_and_in_probation() {
    let r = reputation();
    assert_eq!(r.score(&key(50), 0), 0.5);
    assert_eq!(r.max_parallel(&key(50), 0), 1);
}

#[test]
fn evidence_matches_the_score_it_explains() {
    let mut r = reputation();
    assert!(done(&mut r, 50, 0));
    let (good, bad) = r.evidence(&key(50), 0);
    assert!(good > 0.0 && bad == 0.0);
    let expected = (good + 1.0) / (good + bad + 2.0);
    assert!((r.score(&key(50), 0) - expected).abs() < 1e-12);
    assert_eq!(r.evidence(&key(51), 0), (0.0, 0.0));
}

#[test]
fn successes_raise_the_score_and_the_parallel_allowance() {
    let mut r = reputation();
    for _ in 0..6 {
        assert!(done(&mut r, 50, 0));
    }
    assert!(r.score(&key(50), 0) > 0.85);
    assert!(r.max_parallel(&key(50), 0) > 1);
    assert!(r.max_parallel(&key(50), 0) <= 8, "plafonné");
}

#[test]
fn failures_lower_the_score_and_can_suspend_a_worker() {
    let mut r = reputation();
    for _ in 0..5 {
        r.observe(&key(1), &key(50), &[], Outcome::Failed, 0);
    }
    assert!(r.score(&key(50), 0) < 0.2);
    assert_eq!(r.max_parallel(&key(50), 0), 0);
}

#[test]
fn only_trusted_scopes_count() {
    let mut r = reputation();
    let counted = r.observe(&key(2), &key(50), &[], Outcome::Completed { reward: 1 }, 0);
    assert!(!counted, "un requester inconnu ne fait pas de réputation");
    assert_eq!(r.score(&key(50), 0), 0.5);
}

#[test]
fn self_dealing_is_ignored() {
    let mut r = reputation();
    let committee = [key(10), key(11)];
    let completed = Outcome::Completed { reward: 1_000_000 };
    assert!(
        !r.observe(&key(1), &key(1), &committee, completed, 0),
        "le requester lui-même"
    );
    assert!(
        !r.observe(&key(1), &key(10), &committee, completed, 0),
        "un insider"
    );
    assert_eq!(r.tracked(), 0);
}

#[test]
fn a_huge_reward_cannot_buy_a_reputation_in_one_shot() {
    let mut r = reputation();
    r.observe(
        &key(1),
        &key(50),
        &[],
        Outcome::Completed { reward: u64::MAX },
        0,
    );
    assert!(r.score(&key(50), 0) < 0.8);
    assert_eq!(r.max_parallel(&key(50), 0), 3, "au plus 1 + 2 de poids");
}

#[test]
fn everything_decays_with_the_half_life() {
    let mut r = reputation();
    for _ in 0..8 {
        done(&mut r, 50, 0);
    }
    let fresh = r.score(&key(50), 0);
    let later = r.score(&key(50), 14 * DAY);
    assert!(later < fresh && later > 0.5, "{fresh} -> {later}");
    let long_after = r.score(&key(50), 365 * DAY);
    assert!(
        (long_after - 0.5).abs() < 0.01,
        "retour au neutre : {long_after}"
    );
}

#[test]
fn the_table_is_bounded_and_stale_entries_are_pruned_to_make_room() {
    let config = ReputationConfig {
        max_records: 2,
        ..ReputationConfig::default()
    };
    let mut r = Reputation::new(config, [key(1)]);
    assert!(done(&mut r, 50, 0));
    assert!(done(&mut r, 51, 0));
    assert!(
        !done(&mut r, 52, 0),
        "table pleine d'entrées fraîches : refus"
    );
    assert!(
        done(&mut r, 52, 400 * DAY),
        "les entrées périmées sont purgées"
    );
    assert!(r.tracked() <= 2);
}
