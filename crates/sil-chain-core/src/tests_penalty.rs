//! Pénalité par vote contraire signé (ADR 0012) : seule la majorité est protégée, le requester
//! ne décide rien, la caution est partagée 70 % requester / 30 % majorité.

use crate::reputation::{PENALTY_STEP, VERIFIER_START};
use crate::*;
use sil_types::{Hash32, JobId, PublicKey};

fn key(n: u8) -> PublicKey {
    PublicKey::from_bytes([n; 32])
}

fn terms(verifier_reward_per_unit: u64) -> EscrowTerms {
    EscrowTerms {
        requester: key(100),
        workload: "calcul/sha256".into(),
        input_size: 30,
        width: 0,
        height: 0,
        fragments: 3,
        verifications_required: 3,
        reward_per_unit: 10,
        verifier_reward_per_unit,
        expires_at_ms: 1_000_000,
    }
}

/// Escrow réglé sur le fragment 0 par les approbateurs 70, 71 et 72 (résultat `[7; 32]`).
fn settled(verifier_reward_per_unit: u64) -> Escrow {
    let t = terms(verifier_reward_per_unit);
    let budget = t.max_payout().unwrap();
    let mut e = Escrow::open(JobId::from_bytes([1; 32]), t, budget, &key(100)).unwrap();
    let args = SettleArgs {
        fragment: 0,
        attempt: 1,
        result: Hash32::from_bytes([7; 32]),
        claimant: key(60),
        approvers: vec![key(70), key(71), key(72)],
    };
    let standing = vec![Some(VERIFIER_START); 3];
    e.settle(&args, &[key(70), key(71), key(72)], &key(200), &standing)
        .unwrap();
    e
}

/// Verdict contraire de `verifier` sur le fragment 0, tentative 1 : approbation d'un autre résultat.
fn dissent(verifier: u8) -> PenaltyArgs {
    PenaltyArgs {
        fragment: 0,
        attempt: 1,
        result: Hash32::from_bytes([9; 32]),
        approved: true,
        evidence: None,
        tolerance: None,
        verifier: key(verifier),
    }
}

/// Rejet du résultat réglé (`[7; 32]`), avec sa preuve.
fn rejection(verifier: u8) -> PenaltyArgs {
    PenaltyArgs {
        fragment: 0,
        attempt: 1,
        result: Hash32::from_bytes([7; 32]),
        approved: false,
        evidence: Some(Hash32::from_bytes([5; 32])),
        tolerance: None,
        verifier: key(verifier),
    }
}

#[test]
fn a_signed_dissent_from_outside_the_majority_is_penalized() {
    let mut e = settled(1);
    // Fragment 0 : 10 unités × 1 par vérificateur, dont 5 en réserve.
    let penalty = e
        .penalize(&dissent(80), &[key(80)], Some(VERIFIER_START), 50)
        .unwrap();
    assert_eq!(penalty.score, VERIFIER_START - PENALTY_STEP);
    assert_eq!(penalty.confiscated, 5);
}

#[test]
fn the_requester_cannot_penalize_without_a_signed_dissent() {
    let mut e = settled(1);
    assert_eq!(
        e.penalize(&dissent(80), &[key(100)], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("vote contraire non signé")
    );
    assert_eq!(
        e.penalize(&dissent(80), &[], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("vote contraire non signé")
    );
}

#[test]
fn an_approver_cannot_be_penalized_for_its_own_dissent() {
    let mut e = settled(1);
    assert_eq!(
        e.penalize(&dissent(70), &[key(70)], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("vérificateur dans la majorité")
    );
}

#[test]
fn a_rejection_of_the_settled_result_is_a_dissent_when_it_carries_its_proof() {
    let mut e = settled(1);
    let penalty = e
        .penalize(&rejection(80), &[key(80)], Some(VERIFIER_START), 50)
        .unwrap();
    assert_eq!(penalty.score, VERIFIER_START - PENALTY_STEP);
}

#[test]
fn an_approval_of_the_settled_result_is_not_a_dissent() {
    let mut e = settled(1);
    let mut same = dissent(80);
    same.result = Hash32::from_bytes([7; 32]);
    assert_eq!(
        e.penalize(&same, &[key(80)], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("verdict conforme au règlement")
    );
}

#[test]
fn a_rejection_without_proof_or_an_approval_with_proof_is_malformed() {
    let mut e = settled(1);
    let mut no_proof = rejection(80);
    no_proof.evidence = None;
    assert_eq!(
        e.penalize(&no_proof, &[key(80)], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("preuve manquante")
    );
    let mut with_proof = dissent(80);
    with_proof.evidence = Some(Hash32::from_bytes([5; 32]));
    assert_eq!(
        e.penalize(&with_proof, &[key(80)], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("preuve inattendue")
    );
}

#[test]
fn a_dissent_on_another_attempt_is_refused() {
    let mut e = settled(1);
    let mut other = dissent(80);
    other.attempt = 2;
    assert_eq!(
        e.penalize(&other, &[key(80)], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("tentative différente du règlement")
    );
}

#[test]
fn a_verifier_is_penalized_once_per_fragment() {
    let mut e = settled(1);
    e.penalize(&dissent(80), &[key(80)], Some(VERIFIER_START), 50)
        .unwrap();
    assert_eq!(
        e.penalize(&dissent(80), &[key(80)], Some(VERIFIER_START), 50)
            .unwrap_err(),
        RuleError::BadPenalty("déjà pénalisé pour ce fragment")
    );
}

#[test]
fn a_verifier_without_account_is_not_penalized() {
    let mut e = settled(1);
    assert_eq!(
        e.penalize(&dissent(80), &[key(80)], None, 50).unwrap_err(),
        RuleError::BadPenalty("pas de compte de réputation")
    );
}

#[test]
fn the_confiscation_is_capped_by_the_reserve_left() {
    let mut e = settled(1);
    let penalty = e
        .penalize(&dissent(80), &[key(80)], Some(VERIFIER_START), 2)
        .unwrap();
    assert_eq!(penalty.confiscated, 2, "pas plus que la réserve");
}

#[test]
fn the_split_is_70_percent_requester_and_30_percent_shared_by_the_majority() {
    // Fragment 0 : 10 unités × 100 par vérificateur = 1 000, dont 500 en réserve.
    let mut e = settled(100);
    let penalty = e
        .penalize(&dissent(80), &[key(80)], Some(VERIFIER_START), 10_000)
        .unwrap();
    assert_eq!(penalty.confiscated, 500);
    // 30 % de 500 = 150, soit 50 par approbateur ; le reste (350) au requester.
    assert_eq!(
        penalty.payouts,
        vec![
            Payout {
                to: key(100),
                amount: 350
            },
            Payout {
                to: key(70),
                amount: 50
            },
            Payout {
                to: key(71),
                amount: 50
            },
            Payout {
                to: key(72),
                amount: 50
            },
        ]
    );
    let total: u64 = penalty.payouts.iter().map(|p| p.amount).sum();
    assert_eq!(total, penalty.confiscated, "tout ce qui est confisqué est versé");
}

#[test]
fn an_empty_reserve_still_lowers_the_score_but_moves_no_money() {
    let mut e = settled(100);
    let penalty = e
        .penalize(&dissent(80), &[key(80)], Some(VERIFIER_START), 0)
        .unwrap();
    assert_eq!(penalty.score, VERIFIER_START - PENALTY_STEP);
    assert_eq!(penalty.confiscated, 0);
    assert!(penalty.payouts.iter().all(|p| p.amount == 0));
}
