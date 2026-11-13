//! Pénalité d'un vérificateur qui a voté contre la majorité, sur un validateur local (ADR 0012).
//!
//! Scénario : le dissident approuve le fragment 0 (il gagne une réserve), puis la majorité règle le
//! fragment 1, et le dissident signe un verdict contraire sur ce fragment. La pénalité doit baisser
//! son score, confisquer sa réserve, et verser 70 % au requester et 30 % à la majorité.
#![allow(unused_imports)]

mod common;
use common::*;
use sil_chain_core::reputation::{PENALTY_STEP, VERIFIED_STEP, VERIFIER_START};
use sil_chain_core::wire::attestation_frame;
use sil_chain_core::PenaltyArgs;
use sil_settlement::{Chain, ChainError, PenaltyClaim, TxStatus};
use sil_settlement_solana::{SolanaChain, tx};
use sil_types::{Hash32, PublicKey};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signer};

/// Lit un compte de vérificateur : (score, réserve), au niveau `confirmed`.
fn verifier_state(chain: &SolanaChain, program: &solana_sdk::pubkey::Pubkey, key: &PublicKey) -> (i64, u64) {
    let address = tx::verifier_address(program, key);
    let account = chain
        .rpc()
        .get_account_with_commitment(&address, CommitmentConfig::confirmed())
        .expect("read verifier account")
        .value
        .expect("verifier account exists");
    let score = i64::from_le_bytes(account.data[0..8].try_into().expect("score"));
    let reserved = u64::from_le_bytes(account.data[8..16].try_into().expect("reserve"));
    (score, reserved)
}

#[test]
#[ignore = "needs solana-test-validator, see chain/adapter/README.md"]
fn a_dissenting_verifier_is_penalized_on_chain() {
    let (url, program) = config();
    let payer = Keypair::new();
    let payer_address = payer.pubkey();
    let chain = SolanaChain::new(&url, program, payer);
    let airdrop = chain
        .rpc()
        .request_airdrop(&payer_address, 20 * SOL)
        .expect("airdrop payer");
    wait_for(
        || chain.rpc().confirm_transaction(&airdrop).unwrap_or(false),
        "payer funds",
    );

    let requester = funded(&chain, 5 * SOL);
    let claimant = funded(&chain, SOL);
    let dissenter = funded_identity(&chain, SOL);
    let majority = funded_identity(&chain, SOL);
    let job = random_job();
    let expires = now_ms() + 3_600_000;
    let deposit = terms(&requester, expires).max_payout().expect("budget");
    let tx_open = chain
        .open_escrow(&requester, job, terms(&requester, expires), deposit)
        .expect("open accepted");
    assert_eq!(settle_status(&chain, &tx_open), TxStatus::Confirmed);

    // Fragment 0 : le dissident est approbateur. Il gagne sa réserve et son crédit de réputation.
    let first = signed(claim(job, 0, &claimant, &dissenter), &[&dissenter]);
    let tx_first = chain.submit(&first).expect("first settlement accepted");
    assert_eq!(settle_status(&chain, &tx_first), TxStatus::Confirmed);
    let dissenter_key = dissenter.public();
    let (score_before, reserve_before) = verifier_state(&chain, &program, &dissenter_key);
    assert_eq!(score_before, VERIFIER_START + VERIFIED_STEP, "crédit du règlement");
    assert!(reserve_before > 0, "la réserve vient du fragment 0");

    // Fragment 1 : la majorité règle. Le dissident signe un verdict contraire (autre résultat).
    let second = signed(claim(job, 1, &claimant, &majority), &[&majority]);
    let tx_second = chain.submit(&second).expect("majority settlement accepted");
    assert_eq!(settle_status(&chain, &tx_second), TxStatus::Confirmed);

    let other = Hash32::from_bytes([9; 32]);
    let frame = attestation_frame(&job, 1, 1, &other, true, None);
    let penalty = PenaltyClaim {
        job,
        requester: PublicKey::from_bytes(requester.pubkey().to_bytes()),
        approvers: vec![majority.public()],
        penalty: PenaltyArgs {
            fragment: 1,
            attempt: 1,
            result: other,
            approved: true,
            evidence: None,
            tolerance: None,
            verifier: dissenter_key,
        },
        signature: dissenter.sign_raw(&frame),
    };
    let requester_before = confirmed_balance(&chain, &requester.pubkey());
    let majority_before = confirmed_balance(&chain, &approver_account(&majority));

    let tx_penalty = chain.penalize(&penalty).expect("penalty accepted");
    assert_eq!(settle_status(&chain, &tx_penalty), TxStatus::Confirmed);

    let (score_after, reserve_after) = verifier_state(&chain, &program, &dissenter_key);
    assert_eq!(score_after, score_before - PENALTY_STEP, "score baissé une seule fois");
    assert_eq!(reserve_after, 0, "toute la réserve retenue sur le fragment est confisquée");

    // Confisqué = réserve (50) ; 30 % à la majorité (15), 70 % au requester (35).
    let requester_after = confirmed_balance(&chain, &requester.pubkey());
    let majority_after = confirmed_balance(&chain, &approver_account(&majority));
    assert_eq!(requester_after - requester_before, reserve_before - (reserve_before * 30 / 100));
    assert_eq!(majority_after - majority_before, reserve_before * 30 / 100);

    // Même pénalité une seconde fois : refusée (déjà pénalisé pour ce fragment).
    match chain.penalize(&penalty) {
        Err(ChainError::Rejected(text)) => assert!(text.contains("pénalité refusée"), "{text}"),
        other => panic!("second penalty must be rejected, got {other:?}"),
    }
}
