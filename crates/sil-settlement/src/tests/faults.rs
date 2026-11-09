use super::support::*;
use crate::{
    testing::{FakeChain, Op},
    *,
};

fn setup() -> (FakeChain, Outbox, IntentId) {
    let chain = funded_chain();
    let mut outbox = outbox();
    let id = outbox.enqueue(authorized(claim(0), &[70, 71])).unwrap();
    (chain, outbox, id)
}

#[test]
fn a_rate_limited_rpc_backs_off_then_shows_unknown_then_recovers() {
    let (chain, mut outbox, id) = setup();
    chain.fail(Op::Escrow, ChainError::Transient("429".into()), 3);
    outbox.advance(&chain, 0);
    assert_eq!(outbox.status(&id, 0), Some(Status::Pending));
    assert_eq!(
        outbox.advance(&chain, 500),
        0,
        "dans le délai d'attente : rien"
    );
    outbox.advance(&chain, 61_000);
    outbox.advance(&chain, 122_000);
    assert_eq!(outbox.status(&id, 122_000), Some(Status::Unknown));
    assert_eq!(chain.submit_calls(), 0);
    run(&mut outbox, &chain, 190_000, 61_000, 4);
    assert_eq!(outbox.status(&id, 500_000), Some(Status::Confirmed));
}

#[test]
fn a_transaction_that_landed_despite_an_error_is_never_sent_twice() {
    let (chain, mut outbox, id) = setup();
    chain.land_but_report(ChainError::Transient("délai dépassé".into()));
    outbox.advance(&chain, 0);
    assert_eq!(outbox.status(&id, 0), Some(Status::Pending));
    assert_eq!(
        chain.escrow_now(&job()).unwrap().paid(),
        120 + sil_chain_core::SETTLEMENT_FEE,
        "elle a bien atterri"
    );
    outbox.advance(&chain, 61_000);
    assert_eq!(outbox.status(&id, 61_000), Some(Status::Confirmed));
    assert_eq!(chain.submit_calls(), 1, "pas de second envoi");
    assert_eq!(
        chain.escrow_now(&job()).unwrap().paid(),
        120 + sil_chain_core::SETTLEMENT_FEE
    );
}

#[test]
fn a_lost_transaction_expires_then_is_resent_after_rereading_the_escrow() {
    let (chain, mut outbox, id) = setup();
    chain.drop_next_submit();
    outbox.advance(&chain, 0);
    assert_eq!(outbox.status(&id, 0), Some(Status::Submitted));
    chain.mine();
    outbox.advance(&chain, 2_000);
    assert_eq!(
        outbox.status(&id, 2_000),
        Some(Status::Submitted),
        "toujours en attente"
    );
    chain.advance_height(200); // blockhash expiré
    outbox.advance(&chain, 4_000); // Expired → Pending
    assert_eq!(outbox.status(&id, 4_000), Some(Status::Pending));
    run(&mut outbox, &chain, 6_000, 3_000, 4);
    assert_eq!(outbox.status(&id, 30_000), Some(Status::Confirmed));
    assert_eq!(chain.submit_calls(), 2);
    assert_eq!(
        chain.escrow_now(&job()).unwrap().paid(),
        120 + sil_chain_core::SETTLEMENT_FEE,
        "payé une fois"
    );
}

#[test]
fn a_worker_without_fees_is_blocked_visibly_then_proceeds_when_funded() {
    let (chain, mut outbox, id) = setup();
    chain.fail(Op::Submit, ChainError::NoFunds, 1);
    outbox.advance(&chain, 0);
    assert!(matches!(outbox.status(&id, 0), Some(Status::Blocked(_))));
    run(&mut outbox, &chain, 5_000, 5_000, 4);
    assert_eq!(outbox.status(&id, 25_000), Some(Status::Confirmed));
}

#[test]
fn a_program_refusal_is_final_unless_the_chain_shows_it_was_already_done() {
    let (chain, mut outbox, id) = setup();
    chain.fail(Op::Submit, ChainError::Rejected("règle violée".into()), 1);
    outbox.advance(&chain, 0);
    assert!(matches!(outbox.status(&id, 0), Some(Status::Rejected(_))));
    assert_eq!(chain.escrow_now(&job()).unwrap().paid(), 0);
}

#[test]
fn a_status_outage_while_waiting_does_not_lose_or_duplicate_the_transaction() {
    let (chain, mut outbox, id) = setup();
    outbox.advance(&chain, 0);
    chain.mine();
    chain.fail(Op::Status, ChainError::Transient("rpc".into()), 2);
    outbox.advance(&chain, 2_000);
    outbox.advance(&chain, 70_000);
    assert!(matches!(
        outbox.status(&id, 70_000),
        Some(Status::Submitted | Status::Pending)
    ));
    run(&mut outbox, &chain, 140_000, 70_000, 3);
    assert_eq!(outbox.status(&id, 400_000), Some(Status::Confirmed));
    assert_eq!(chain.submit_calls(), 1);
}

#[test]
fn a_restart_between_submission_and_confirmation_loses_and_duplicates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let file = OutboxFile::new(dir.path().join("sub").join("outbox.cbor"));
    let (chain, mut outbox, id) = setup();
    outbox.advance(&chain, 0);
    file.save(&outbox).unwrap();
    drop(outbox); // le nœud s'arrête ici

    chain.mine();
    let mut restarted = file.load(OutboxConfig::default()).unwrap();
    assert_eq!(restarted.status(&id, 0), Some(Status::Submitted));
    restarted.advance(&chain, 5_000);
    assert_eq!(restarted.status(&id, 5_000), Some(Status::Confirmed));
    assert_eq!(
        chain.submit_calls(),
        1,
        "aucun nouvel envoi après redémarrage"
    );
}

#[test]
fn a_restart_after_a_crash_before_saving_resubmits_safely() {
    // Le nœud a soumis puis s'est arrêté avant d'avoir sauvegardé : au redémarrage il ne
    // sait pas qu'il a soumis. La relecture de l'escrow évite le double paiement.
    let (chain, mut first, _) = setup();
    first.advance(&chain, 0);
    chain.mine();
    let mut fresh = outbox();
    let id = fresh.enqueue(authorized(claim(0), &[70, 71])).unwrap();
    fresh.advance(&chain, 10_000);
    assert_eq!(fresh.status(&id, 10_000), Some(Status::Confirmed));
    assert_eq!(chain.submit_calls(), 1);
    assert_eq!(
        chain.escrow_now(&job()).unwrap().paid(),
        120 + sil_chain_core::SETTLEMENT_FEE
    );
}

#[test]
fn a_corrupt_outbox_file_is_an_error_not_an_empty_outbox() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outbox.cbor");
    std::fs::write(&path, b"\xff\xfe not cbor").unwrap();
    let result = OutboxFile::new(&path).load(OutboxConfig::default());
    assert!(matches!(result, Err(SettlementError::Corrupt(_))));
    assert!(
        OutboxFile::new(dir.path().join("absent"))
            .load(OutboxConfig::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_refused_submit_backs_off_instead_of_retrying_every_step() {
    let (chain, mut outbox, id) = setup();
    // La lecture réussit à chaque fois, mais la soumission est refusée : il ne doit pas y avoir un
    // nouvel envoi à chaque pas (le délai doit grandir).
    chain.fail(
        Op::Submit,
        ChainError::Transient("règle en attente".into()),
        100,
    );
    let mut now = 0;
    for _ in 0..60 {
        outbox.advance(&chain, now);
        now += 1_000;
    }
    assert!(
        chain.submit_calls() <= 8,
        "trop de soumissions : {}",
        chain.submit_calls()
    );
    assert_ne!(outbox.status(&id, now), Some(Status::Confirmed));
}
