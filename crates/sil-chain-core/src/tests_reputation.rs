//! Gate de réputation au règlement (ADR 0006) et signatures des approbateurs (ADR 0007).

use crate::reputation::{VERIFIED_STEP, VERIFIER_ELIGIBLE, VERIFIER_START};
use crate::*;
use sil_types::{Hash32, JobId, PublicKey};

fn key(n: u8) -> PublicKey {
    PublicKey::from_bytes([n; 32])
}

fn terms() -> EscrowTerms {
    EscrowTerms {
        requester: key(100),
        workload: "calcul/sha256".into(),
        input_size: 30,
        width: 0,
        height: 0,
        fragments: 3,
        verifications_required: 3,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: 1_000_000,
    }
}

fn escrow() -> Escrow {
    let budget = terms().max_payout().unwrap();
    Escrow::open(JobId::from_bytes([1; 32]), terms(), budget, &key(100)).unwrap()
}

/// Approbateurs 70 et 71 : `args(0)` les désigne.
fn args(fragment: u32) -> SettleArgs {
    SettleArgs {
        fragment,
        attempt: 1,
        result: Hash32::from_bytes([7; 32]),
        claimant: key(60),
        approvers: vec![key(70), key(71), key(72)],
    }
}

/// Les trois approbateurs ont signé.
fn signed() -> Vec<PublicKey> {
    vec![key(70), key(71), key(72)]
}

fn eligible() -> Vec<Option<i64>> {
    vec![Some(VERIFIER_START); 3]
}

#[test]
fn an_approver_without_an_account_blocks_the_settlement() {
    let mut e = escrow();
    let standing = vec![Some(VERIFIER_START), Some(VERIFIER_START), None];
    assert_eq!(
        e.settle(&args(0), &signed(), &key(200), &standing)
            .unwrap_err(),
        RuleError::IneligibleVerifier {
            have: None,
            need: VERIFIER_ELIGIBLE
        }
    );
    assert_eq!(e.settled_count(), 0, "rien n'est réglé");
}

#[test]
fn an_approver_under_the_threshold_blocks_the_settlement() {
    let mut e = escrow();
    let standing = vec![
        Some(VERIFIER_START),
        Some(VERIFIER_START),
        Some(VERIFIER_ELIGIBLE - 1),
    ];
    assert!(matches!(
        e.settle(&args(0), &signed(), &key(200), &standing),
        Err(RuleError::IneligibleVerifier { .. })
    ));
}

#[test]
fn a_settlement_credits_every_approver_once() {
    let mut e = escrow();
    let done = e
        .settle(&args(0), &signed(), &key(200), &eligible())
        .unwrap();
    assert_eq!(
        done.credited,
        vec![
            (key(70), VERIFIER_START + VERIFIED_STEP),
            (key(71), VERIFIER_START + VERIFIED_STEP),
            (key(72), VERIFIER_START + VERIFIED_STEP),
        ]
    );
    assert!(
        done.payouts.iter().any(|p| p.to == key(70)),
        "les approbateurs restent payés"
    );
}

#[test]
fn a_length_mismatch_on_standing_is_refused() {
    let mut e = escrow();
    assert_eq!(
        e.settle(&args(0), &signed(), &key(200), &[Some(VERIFIER_START)])
            .unwrap_err(),
        RuleError::BadApprovers("réputation absente")
    );
}

#[test]
fn an_approver_who_did_not_sign_blocks_the_settlement() {
    let mut e = escrow();
    assert_eq!(
        e.settle(&args(0), &[key(70)], &key(200), &eligible())
            .unwrap_err(),
        RuleError::NotAuthorized { have: 1, need: 3 }
    );
    assert_eq!(e.paid(), 0, "rien n'est payé");
}

#[test]
fn the_settlement_frame_does_not_list_the_approvers() {
    let job = JobId::from_bytes([1; 32]);
    let mut other = args(0);
    other.approvers = vec![key(80), key(81)];
    assert_eq!(
        wire::settlement_frame(&job, &args(0)),
        wire::settlement_frame(&job, &other),
        "les approbateurs ne sont connus qu'à la finalisation"
    );
    assert!(wire::settlement_frame(&job, &args(0)).starts_with(wire::SETTLEMENT_DOMAIN));
}
