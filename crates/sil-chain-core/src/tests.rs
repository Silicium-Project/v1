use crate::SETTLEMENT_FEE;
use crate::*;
use sil_types::{Hash32, JobId, PublicKey};

fn key(n: u8) -> PublicKey {
    PublicKey::from_bytes([n; 32])
}

fn terms() -> EscrowTerms {
    EscrowTerms {
        requester: key(100),
        committee: vec![key(10), key(11), key(12)],
        threshold: 2,
        workload: "calcul/sha256".into(),
        input_size: 30,
        width: 0,
        height: 0,
        fragments: 3,
        verifications_required: 2,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: 1_000_000,
    }
}

/// Un fragment de 10 unités : 10 × (10 + 2×1) de récompenses, plus les frais de règlement.
const PER_FRAGMENT: u64 = 120 + SETTLEMENT_FEE;
/// 3 fragments × (récompenses + frais) : le dépôt qui couvre tout.
const BUDGET: u64 = 3 * PER_FRAGMENT;

fn escrow(deposit: u64) -> Escrow {
    Escrow::open(JobId::from_bytes([1; 32]), terms(), deposit, &key(100)).unwrap()
}

fn args(fragment: u32) -> SettleArgs {
    SettleArgs {
        fragment,
        attempt: 1,
        result: Hash32::from_bytes([7; 32]),
        claimant: key(60),
        approvers: vec![key(70), key(71)],
    }
}

const QUORUM: [fn() -> PublicKey; 2] = [|| key(10), || key(11)];

fn signers() -> Vec<PublicKey> {
    QUORUM.iter().map(|k| k()).collect()
}

#[test]
fn opening_requires_the_requester_a_valid_committee_and_full_funding() {
    let job = JobId::from_bytes([1; 32]);
    assert_eq!(terms().max_payout().unwrap(), BUDGET);
    assert!(Escrow::open(job, terms(), BUDGET, &key(100)).is_ok());
    assert_eq!(
        Escrow::open(job, terms(), BUDGET - 1, &key(100)).unwrap_err(),
        RuleError::DepositTooLow {
            have: BUDGET - 1,
            need: BUDGET
        }
    );
    assert_eq!(
        Escrow::open(job, terms(), BUDGET, &key(5)).unwrap_err(),
        RuleError::NotRequester
    );
    let mut weak = terms();
    weak.threshold = 1; // pas une majorité de 3
    assert!(matches!(
        Escrow::open(job, weak, BUDGET, &key(100)),
        Err(RuleError::BadTerms(_))
    ));
    let mut duplicate = terms();
    duplicate.committee = vec![key(10), key(10), key(11)];
    assert!(matches!(
        Escrow::open(job, duplicate, BUDGET, &key(100)),
        Err(RuleError::BadTerms(_))
    ));
    let mut huge = terms();
    huge.reward_per_unit = u64::MAX;
    assert_eq!(
        Escrow::open(job, huge, u64::MAX, &key(100)).unwrap_err(),
        RuleError::Overflow
    );
}

#[test]
fn settling_pays_the_claimant_and_the_verifiers_once() {
    let mut e = escrow(BUDGET);
    let payouts = e.settle(&args(0), &signers(), &key(200)).unwrap();
    assert_eq!(payouts.iter().map(|p| p.amount).sum::<u64>(), PER_FRAGMENT);
    assert_eq!(
        payouts[0],
        Payout {
            to: key(60),
            amount: 100
        }
    );
    assert_eq!(e.paid(), PER_FRAGMENT);
    assert_eq!(e.balance(), BUDGET - PER_FRAGMENT);
    assert_eq!(e.settled(0).unwrap().result, Hash32::from_bytes([7; 32]));
}

#[test]
fn a_fragment_cannot_be_settled_twice_and_the_error_says_if_it_was_the_same() {
    let mut e = escrow(BUDGET);
    e.settle(&args(1), &signers(), &key(200)).unwrap();
    assert_eq!(
        e.settle(&args(1), &signers(), &key(200)).unwrap_err(),
        RuleError::AlreadySettled { same: true }
    );
    let mut other = args(1);
    other.result = Hash32::from_bytes([8; 32]);
    assert_eq!(
        e.settle(&other, &signers(), &key(200)).unwrap_err(),
        RuleError::AlreadySettled { same: false }
    );
    assert_eq!(e.paid(), PER_FRAGMENT, "aucun double paiement");
}

#[test]
fn settlement_needs_distinct_committee_members_up_to_the_threshold() {
    let mut e = escrow(BUDGET);
    let one = [key(10)];
    assert_eq!(
        e.settle(&args(0), &one, &key(200)).unwrap_err(),
        RuleError::NotAuthorized { have: 1, need: 2 }
    );
    let repeated = [key(10), key(10), key(10)];
    assert_eq!(
        e.settle(&args(0), &repeated, &key(200)).unwrap_err(),
        RuleError::NotAuthorized { have: 1, need: 2 }
    );
    let outsiders = [key(10), key(90), key(91)];
    assert!(matches!(
        e.settle(&args(0), &outsiders, &key(200)),
        Err(RuleError::NotAuthorized { have: 1, .. })
    ));
    assert_eq!(e.paid(), 0);
    assert!(e.settle(&args(0), &[key(11), key(12)], &key(200)).is_ok());
}

#[test]
fn approvers_are_exact_distinct_and_independent_of_the_claimant() {
    let mut e = escrow(BUDGET);
    let mut few = args(0);
    few.approvers.pop();
    assert!(matches!(
        e.settle(&few, &signers(), &key(200)),
        Err(RuleError::BadApprovers("nombre"))
    ));
    let mut twice = args(0);
    twice.approvers = vec![key(70), key(70)];
    assert!(matches!(
        e.settle(&twice, &signers(), &key(200)),
        Err(RuleError::BadApprovers("doublons"))
    ));
    let mut selfie = args(0);
    selfie.approvers = vec![key(60), key(71)];
    assert!(matches!(
        e.settle(&selfie, &signers(), &key(200)),
        Err(RuleError::BadApprovers(_))
    ));
    let mut bad_attempt = args(0);
    bad_attempt.attempt = 0;
    assert_eq!(
        e.settle(&bad_attempt, &signers(), &key(200)).unwrap_err(),
        RuleError::BadAttempt
    );
    assert_eq!(
        e.settle(&args(3), &signers(), &key(200)).unwrap_err(),
        RuleError::UnknownFragment
    );
    assert_eq!(e.paid(), 0);
}

#[test]
fn all_fragments_together_exhaust_the_balance_exactly() {
    let mut e = escrow(BUDGET);
    for fragment in 0..3 {
        e.settle(&args(fragment), &signers(), &key(200)).unwrap();
    }
    assert_eq!(e.balance(), 0);
    assert_eq!(
        e.settle(&args(0), &signers(), &key(200)).unwrap_err(),
        RuleError::AlreadySettled { same: true }
    );
}

#[test]
fn refund_is_only_for_the_requester_after_expiry_and_grace() {
    let mut e = escrow(BUDGET + 40);
    e.settle(&args(0), &signers(), &key(200)).unwrap();
    let opens = 1_000_000 + REFUND_GRACE_MS;
    assert_eq!(
        e.refund(&key(60), opens).unwrap_err(),
        RuleError::NotRequester
    );
    assert_eq!(
        e.refund(&key(100), 999_999).unwrap_err(),
        RuleError::TooEarlyToRefund
    );
    assert_eq!(
        e.refund(&key(100), opens - 1).unwrap_err(),
        RuleError::TooEarlyToRefund
    );
    assert_eq!(
        e.refund(&key(100), opens).unwrap(),
        BUDGET + 40 - PER_FRAGMENT,
        "budget + 40 - un fragment payé"
    );
    assert!(e.is_closed());
    assert_eq!(e.balance(), 0);
    assert_eq!(e.refund(&key(100), opens).unwrap_err(), RuleError::Refunded);
}

#[test]
fn nothing_is_settled_after_a_refund_and_top_up_stops() {
    let mut e = escrow(BUDGET);
    e.refund(&key(100), 1_000_000 + REFUND_GRACE_MS).unwrap();
    assert_eq!(
        e.settle(&args(0), &signers(), &key(200)).unwrap_err(),
        RuleError::Refunded
    );
    assert_eq!(e.top_up(&key(100), 5).unwrap_err(), RuleError::Refunded);
}

#[test]
fn top_up_is_requester_only_and_overflow_safe() {
    let mut e = escrow(BUDGET);
    assert_eq!(e.top_up(&key(60), 5).unwrap_err(), RuleError::NotRequester);
    e.top_up(&key(100), 40).unwrap();
    assert_eq!(e.balance(), BUDGET + 40);
    assert_eq!(
        e.top_up(&key(100), u64::MAX).unwrap_err(),
        RuleError::Overflow
    );
}

/// Suite pseudo-aléatoire déterministe (xorshift).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn random_operations_never_break_the_money_invariants() {
    for seed in 1..=40u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut e = escrow(BUDGET + rng.next() % 200);
        let mut now = 0u64;
        for _ in 0..300 {
            now += rng.next() % 400_000;
            match rng.next() % 5 {
                0 | 1 => {
                    let mut a = args((rng.next() % 5) as u32);
                    a.attempt = (rng.next() % 18) as u32;
                    a.result = Hash32::from_bytes([(rng.next() % 3) as u8; 32]);
                    let members: Vec<PublicKey> = (0..(rng.next() % 5))
                        .map(|_| key(10 + (rng.next() % 5) as u8))
                        .collect();
                    let _ = e.settle(&a, &members, &key(200));
                }
                2 => {
                    let signer = if rng.next().is_multiple_of(4) {
                        key(60)
                    } else {
                        key(100)
                    };
                    let _ = e.refund(&signer, now);
                }
                3 => {
                    let _ = e.top_up(&key(100), rng.next() % 50);
                }
                _ => {}
            }
            assert!(e.paid() + e.refunded() <= e.deposited(), "graine {seed}");
            assert_eq!(e.balance(), e.deposited() - e.paid() - e.refunded());
            assert!(e.settled_count() <= 3);
            assert_eq!(
                e.paid(),
                PER_FRAGMENT * e.settled_count() as u64,
                "graine {seed}"
            );
            if e.is_closed() {
                assert_eq!(e.balance(), 0);
            }
        }
    }
}
