//! Escrow lifecycle on a real validator (see chain/adapter/README.md).
#![allow(unused_imports)]

mod common;
use common::*;
use sil_chain_core::{REFUND_GRACE_MS, SETTLEMENT_FEE};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_settlement::{Authorization, Chain, ChainError, SettlementClaim, TxStatus};
use sil_settlement::{Outbox, OutboxConfig, Status};
use sil_settlement_solana::SolanaChain;
use sil_types::{Hash32, JobId, PublicKey};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::time::Duration;

#[test]
#[ignore = "needs solana-test-validator, see chain/adapter/README.md"]
fn escrow_lifecycle_on_a_local_validator() {
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
    let approver = funded(&chain, SOL);
    let members = [sil_key(21), sil_key(22), sil_key(23)];
    let committee: Vec<PublicKey> = members.iter().map(SigningIdentity::public).collect();
    let job = random_job();
    let expires = now_ms() + 3_600_000;
    let deposit = terms(&requester, &committee, expires)
        .max_payout()
        .expect("budget");

    // Ouverture : l'escrow existe avec le dépôt et les conditions voulues.
    let tx = chain
        .open_escrow(
            &requester,
            job,
            terms(&requester, &committee, expires),
            deposit,
        )
        .expect("open accepted");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed);
    let escrow = chain.escrow(&job).expect("read").expect("escrow exists");
    assert_eq!(escrow.deposited(), deposit);
    assert_eq!(escrow.terms.committee, committee);

    // Règlement d'un fragment avec deux signatures du comité : payé une fois.
    let before = confirmed_balance(&chain, &claimant.pubkey());
    let auth = signed(
        claim(job, 0, &claimant, &approver),
        &[&members[0], &members[1]],
    );
    let tx = chain.submit(&auth).expect("settlement accepted");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed);
    let escrow = chain.escrow(&job).expect("read").expect("escrow exists");
    assert!(escrow.settled(0).is_some(), "fragment 0 settled on chain");
    assert_eq!(escrow.paid(), 1_100 + SETTLEMENT_FEE);
    let after = confirmed_balance(&chain, &claimant.pubkey());
    assert_eq!(after - before, 1_000, "the claimant got the reward");
    let approver_before = confirmed_balance(&chain, &approver.pubkey());
    assert!(approver_before > 0);

    // Même règlement une seconde fois : refusé par le programme (déjà réglé).
    match chain.submit(&auth) {
        Err(ChainError::Rejected(text)) => assert!(text.contains("déjà réglé"), "{text}"),
        other => panic!("second settlement must be rejected, got {other:?}"),
    }

    // Une seule signature : pas assez de membres du comité.
    let weak = signed(claim(job, 1, &claimant, &approver), &[&members[0]]);
    match chain.submit(&weak) {
        Err(ChainError::Rejected(text)) => {
            assert!(text.contains("autorisation insuffisante"), "{text}")
        }
        other => panic!("one signature must not settle, got {other:?}"),
    }

    // Signature d'un non-membre du comité : elle ne compte pas.
    let outsider = signed(
        claim(job, 1, &claimant, &approver),
        &[&members[0], &sil_key(99)],
    );
    match chain.submit(&outsider) {
        Err(ChainError::Rejected(text)) => {
            assert!(text.contains("autorisation insuffisante"), "{text}")
        }
        other => panic!("a non-member signature must not count, got {other:?}"),
    }

    // Ajout de fonds : le dépôt augmente.
    let tx = chain.top_up(&requester, job, 500).expect("top-up accepted");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed);
    assert_eq!(
        chain
            .escrow(&job)
            .expect("read")
            .expect("exists")
            .deposited(),
        deposit + 500
    );

    // Remboursement avant l'échéance plus le délai de grâce : refusé.
    match chain.refund(&requester, job) {
        Err(ChainError::Rejected(text)) => assert!(text.contains("trop tôt"), "{text}"),
        other => panic!("refund before expiry must be refused, got {other:?}"),
    }
}

#[test]
#[ignore = "needs solana-test-validator, see chain/adapter/README.md"]
fn an_expired_escrow_refunds_its_balance_and_cannot_be_reopened() {
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
    let committee: Vec<PublicKey> = [sil_key(31), sil_key(32), sil_key(33)]
        .iter()
        .map(SigningIdentity::public)
        .collect();
    let job = random_job();
    // Expired more than the grace period ago: the refund is due now.
    let expires = now_ms().saturating_sub(REFUND_GRACE_MS + 3_600_000);
    let deposit = terms(&requester, &committee, expires)
        .max_payout()
        .expect("budget");
    let tx = chain
        .open_escrow(
            &requester,
            job,
            terms(&requester, &committee, expires),
            deposit,
        )
        .expect("open accepted");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed);

    // The refund fee is paid by the requester, so the check is on the escrow account itself.
    let address = sil_settlement_solana::escrow_address(&program, &job);
    let account_before = confirmed_balance(&chain, &address);
    let tx = chain.refund(&requester, job).expect("refund accepted");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed);
    let escrow = chain.escrow(&job).expect("read").expect("account kept");
    assert!(escrow.is_closed());
    assert_eq!(escrow.refunded(), deposit);
    let account_after = confirmed_balance(&chain, &address);
    assert_eq!(
        account_before - account_after,
        deposit,
        "the balance left the escrow account"
    );
    assert!(
        account_after > 0,
        "the rent stays locked so the job cannot be reopened"
    );

    // The same job cannot be opened again: the closed account stays.
    match chain.open_escrow(
        &requester,
        job,
        terms(&requester, &committee, expires),
        deposit,
    ) {
        Err(ChainError::Rejected(_)) => {}
        other => panic!("reopening a refunded job must fail, got {other:?}"),
    }
}
