//! Resilience scenarios on a real validator: no SOL, two nodes racing, unreachable RPC.
#![allow(unused_imports)]

mod common;
use common::*;
use sil_chain_core::SETTLEMENT_FEE;
use sil_crypto::SigningIdentity;
use sil_settlement::{Chain, ChainError, Outbox, OutboxConfig, Status, TxStatus};
use sil_settlement_solana::SolanaChain;
use sil_types::PublicKey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::time::Duration;

/// A payer with no SOL is refused before anything is sent (`NoFunds`), so the outbox shows the
/// job as blocked instead of retrying a transaction that cannot pay its fee.
#[test]
#[ignore = "needs solana-test-validator, see chain/adapter/README.md"]
fn a_payer_without_sol_is_refused_before_sending() {
    let (url, program) = config();
    let chain = SolanaChain::new(&url, program, Keypair::new());
    let requester = funded(&chain, 5 * SOL);
    let members = [sil_key(41), sil_key(42), sil_key(43)];
    let committee: Vec<PublicKey> = members.iter().map(SigningIdentity::public).collect();
    let job = random_job();
    let open_terms = terms(&requester, &committee, now_ms() + 3_600_000);
    let budget = open_terms.max_payout().expect("budget");
    let tx = chain
        .open_escrow(&requester, job, open_terms, budget)
        .expect("open accepted");
    assert_eq!(settle_status(&chain, &tx), TxStatus::Confirmed);

    let claimant = funded(&chain, SOL);
    let approver = funded(&chain, SOL);
    let auth = signed(
        claim(job, 0, &claimant, &approver),
        &[&members[0], &members[1]],
    );
    // A chain client whose payer has never been funded.
    let broke = SolanaChain::new(&url, program, Keypair::new());
    assert_eq!(broke.submit(&auth).unwrap_err(), ChainError::NoFunds);
}

/// Two nodes race to settle the same fragment. It is paid exactly once, and both outboxes end up
/// confirmed: the one whose transaction is refused re-reads the escrow and sees it is done.
#[test]
#[ignore = "needs solana-test-validator, see chain/adapter/README.md"]
fn two_nodes_racing_to_settle_pay_exactly_once() {
    let (url, program) = config();
    let payer_a = Keypair::new();
    let payer_b = Keypair::new();
    let (address_a, address_b) = (payer_a.pubkey(), payer_b.pubkey());
    let chain_a = SolanaChain::new(&url, program, payer_a);
    let chain_b = SolanaChain::new(&url, program, payer_b);
    for address in [address_a, address_b] {
        let airdrop = chain_a
            .rpc()
            .request_airdrop(&address, 5 * SOL)
            .expect("airdrop payer");
        wait_for(
            || chain_a.rpc().confirm_transaction(&airdrop).unwrap_or(false),
            "payer funds",
        );
    }

    let requester = funded(&chain_a, 5 * SOL);
    let members = [sil_key(51), sil_key(52), sil_key(53)];
    let committee: Vec<PublicKey> = members.iter().map(SigningIdentity::public).collect();
    let job = random_job();
    let open_terms = terms(&requester, &committee, now_ms() + 3_600_000);
    let budget = open_terms.max_payout().expect("budget");
    let tx = chain_a
        .open_escrow(&requester, job, open_terms, budget)
        .expect("open accepted");
    assert_eq!(settle_status(&chain_a, &tx), TxStatus::Confirmed);

    let claimant = funded(&chain_a, SOL);
    let approver = funded(&chain_a, SOL);
    let auth = signed(
        claim(job, 0, &claimant, &approver),
        &[&members[0], &members[1]],
    );
    let mut outbox_a = Outbox::new(OutboxConfig::default());
    let mut outbox_b = Outbox::new(OutboxConfig::default());
    let id = outbox_a.enqueue(auth.clone()).expect("queued");
    outbox_b.enqueue(auth).expect("queued");

    let mut now = 0;
    for _ in 0..120 {
        now += 1_000;
        outbox_a.advance(&chain_a, now);
        outbox_b.advance(&chain_b, now);
        if outbox_a.status(&id, now) == Some(Status::Confirmed)
            && outbox_b.status(&id, now) == Some(Status::Confirmed)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    assert_eq!(outbox_a.status(&id, now), Some(Status::Confirmed));
    assert_eq!(outbox_b.status(&id, now), Some(Status::Confirmed));
    let escrow = chain_a.escrow(&job).expect("read").expect("exists");
    assert_eq!(
        escrow.paid(),
        1_100 + SETTLEMENT_FEE,
        "paid once, not twice"
    );
}

/// A node that cannot reach its RPC gets a transient error, which the outbox retries later.
#[test]
fn an_unreachable_rpc_is_transient() {
    let chain = SolanaChain::new("http://127.0.0.1:1", Pubkey::new_unique(), Keypair::new());
    assert!(matches!(
        chain.escrow(&random_job()),
        Err(ChainError::Transient(_))
    ));
}
