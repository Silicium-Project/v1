//! The on-chain program rebuilds the settlement message from its arguments
//! (`sil_chain_core::wire::settlement_frame`). It must be byte-identical to what the approvers
//! signs (`SettlementClaim::bytes` under the settlement domain), or no signature would verify.

use crate::SettlementClaim;
use sil_chain_core::wire::{SETTLEMENT_DOMAIN, settlement_frame};
use sil_types::{Hash32, JobId, PublicKey};

#[test]
fn the_program_frame_is_the_message_the_approvers_sign() {
    let claim = SettlementClaim {
        job: JobId::from_bytes([3; 32]),
        fragment: 5,
        attempt: 2,
        result: Hash32::from_bytes([9; 32]),
        claimant: PublicKey::from_bytes([4; 32]),
        approvers: vec![
            PublicKey::from_bytes([6; 32]),
            PublicKey::from_bytes([7; 32]),
        ],
    };
    let mut expected = SETTLEMENT_DOMAIN.to_vec();
    expected.extend(claim.bytes());
    assert_eq!(settlement_frame(&claim.job, &claim.args()), expected);
}

#[test]
fn every_error_code_has_a_description() {
    use sil_chain_core::RuleError;
    use sil_chain_core::wire::{CODE_INVALID_ACCOUNT, describe_code, rule_code};
    let errors = [
        RuleError::NotRequester,
        RuleError::BadTerms("x"),
        RuleError::DepositTooLow { have: 0, need: 1 },
        RuleError::Refunded,
        RuleError::UnknownFragment,
        RuleError::AlreadySettled { same: true },
        RuleError::BadAttempt,
        RuleError::BadApprovers("x"),
        RuleError::NotAuthorized { have: 0, need: 2 },
        RuleError::Insufficient,
        RuleError::TooEarlyToRefund,
        RuleError::NothingToRefund,
        RuleError::Overflow,
        RuleError::IneligibleVerifier {
            have: None,
            need: 2_500,
        },
    ];
    for error in &errors {
        assert_ne!(
            describe_code(rule_code(error)),
            "erreur inconnue du programme"
        );
    }
    assert_ne!(
        describe_code(CODE_INVALID_ACCOUNT),
        "erreur inconnue du programme"
    );
}
