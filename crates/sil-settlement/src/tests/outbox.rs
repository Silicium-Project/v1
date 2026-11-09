use super::support::*;
use crate::{testing::FakeChain, *};
use sil_chain_core::SettleArgs;

#[test]
fn a_settlement_goes_from_pending_to_confirmed_and_pays_once() {
    let chain = funded_chain();
    let mut outbox = outbox();
    let id = outbox.enqueue(authorized(claim(0), &[70, 71])).unwrap();
    assert_eq!(outbox.status(&id, 0), Some(Status::Pending));
    outbox.advance(&chain, 0);
    assert_eq!(outbox.status(&id, 0), Some(Status::Submitted));
    chain.mine();
    outbox.advance(&chain, 2_000);
    assert_eq!(outbox.status(&id, 2_000), Some(Status::Confirmed));
    let escrow = chain.escrow_now(&job()).unwrap();
    assert_eq!(escrow.paid(), 120 + sil_chain_core::SETTLEMENT_FEE);
    assert_eq!(chain.submit_calls(), 1);
    // Rejouer ne change rien : l'état terminal est stable.
    assert_eq!(outbox.advance(&chain, 10_000), 0);
    assert_eq!(chain.submit_calls(), 1);
}

#[test]
fn enqueueing_the_same_settlement_again_merges_signatures_instead_of_duplicating() {
    let mut outbox = outbox();
    let first = outbox.enqueue(authorized(claim(0), &[70])).unwrap();
    let second = outbox.enqueue(authorized(claim(0), &[71])).unwrap();
    let third = outbox.enqueue(authorized(claim(0), &[70, 71])).unwrap();
    assert_eq!(first, second);
    assert_eq!(second, third);
    assert_eq!(outbox.len(), 1);
    // Les trois signatures sont fusionnées : le règlement est autorisé.
    let chain = funded_chain();
    outbox.advance(&chain, 0);
    assert_eq!(outbox.status(&first, 0), Some(Status::Submitted));
}

#[test]
fn nothing_is_submitted_until_every_approver_has_signed() {
    let chain = funded_chain();
    let mut outbox = outbox();
    let c = claim(0);
    let id = outbox.enqueue(authorized(c.clone(), &[70])).unwrap();
    outbox.advance(&chain, 0);
    outbox.advance(&chain, 5_000);
    assert_eq!(chain.submit_calls(), 0, "une signature sur deux requises");
    assert_eq!(outbox.status(&id, 5_000), Some(Status::Pending));

    let second = ident(71);
    assert!(
        outbox
            .add_signature(&id, second.public(), c.sign(&second))
            .unwrap()
    );
    outbox.advance(&chain, 10_000);
    assert_eq!(outbox.status(&id, 10_000), Some(Status::Submitted));
}

#[test]
fn a_fragment_already_settled_as_wanted_is_confirmed_without_any_submission() {
    let chain = funded_chain();
    let mut escrow = chain.escrow_now(&job()).unwrap();
    let c = claim(0);
    let signers: Vec<_> = [70, 71].iter().map(|n| ident(*n).public()).collect();
    escrow
        .settle(
            &c.args(),
            &signers,
            &sil_types::PublicKey::from_bytes([0xEE; 32]),
            &[Some(sil_chain_core::reputation::VERIFIER_START); 2],
        )
        .unwrap();
    chain.insert_escrow(escrow);

    let mut outbox = outbox();
    let id = outbox.enqueue(authorized(c, &[70, 71])).unwrap();
    outbox.advance(&chain, 0);
    assert_eq!(outbox.status(&id, 0), Some(Status::Confirmed));
    assert_eq!(chain.submit_calls(), 0);
}

#[test]
fn a_fragment_settled_with_other_parameters_is_rejected_for_good() {
    let chain = funded_chain();
    let mut escrow = chain.escrow_now(&job()).unwrap();
    let mut other = claim(0);
    other.result = sil_types::Hash32::from_bytes([9; 32]);
    let signers: Vec<_> = [70, 71].iter().map(|n| ident(*n).public()).collect();
    escrow
        .settle(
            &other.args(),
            &signers,
            &sil_types::PublicKey::from_bytes([0xEE; 32]),
            &[Some(sil_chain_core::reputation::VERIFIER_START); 2],
        )
        .unwrap();
    chain.insert_escrow(escrow);

    let mut outbox = outbox();
    let id = outbox.enqueue(authorized(claim(0), &[70, 71])).unwrap();
    outbox.advance(&chain, 0);
    assert!(matches!(outbox.status(&id, 0), Some(Status::Rejected(_))));
    assert_eq!(chain.submit_calls(), 0);
}

#[test]
fn a_missing_escrow_blocks_visibly_then_recovers_when_it_appears() {
    let chain = FakeChain::new();
    let mut outbox = outbox();
    let id = outbox.enqueue(authorized(claim(0), &[70, 71])).unwrap();
    outbox.advance(&chain, 0);
    assert!(matches!(outbox.status(&id, 0), Some(Status::Blocked(_))));
    let escrow = sil_chain_core::Escrow::open(
        job(),
        terms(),
        2 * (120 + sil_chain_core::SETTLEMENT_FEE),
        &ident(100).public(),
    )
    .unwrap();
    chain.insert_escrow(escrow);
    run(&mut outbox, &chain, 60_000, 5_000, 4);
    assert_eq!(outbox.status(&id, 80_000), Some(Status::Confirmed));
}

#[test]
fn two_nodes_racing_to_settle_the_same_fragment_pay_exactly_once() {
    let chain = funded_chain();
    let (mut a, mut b) = (outbox(), outbox());
    let (id_a, id_b) = (
        a.enqueue(authorized(claim(0), &[70, 71])).unwrap(),
        b.enqueue(authorized(claim(0), &[70, 71])).unwrap(),
    );
    assert_eq!(id_a, id_b, "même règlement, même identifiant");
    a.advance(&chain, 0);
    b.advance(&chain, 0);
    chain.mine(); // les deux transactions atterrissent, la seconde échoue côté programme
    run(&mut a, &chain, 5_000, 5_000, 3);
    run(&mut b, &chain, 5_000, 5_000, 3);
    assert_eq!(a.status(&id_a, 20_000), Some(Status::Confirmed));
    assert_eq!(b.status(&id_b, 20_000), Some(Status::Confirmed));
    assert_eq!(
        chain.escrow_now(&job()).unwrap().paid(),
        120 + sil_chain_core::SETTLEMENT_FEE,
        "payé une seule fois"
    );
}

#[test]
fn the_outbox_is_bounded() {
    let mut outbox = Outbox::new(OutboxConfig {
        max_entries: 1,
        ..OutboxConfig::default()
    });
    outbox.enqueue(authorized(claim(0), &[70])).unwrap();
    assert!(matches!(
        outbox.enqueue(authorized(claim(1), &[70])),
        Err(SettlementError::Full)
    ));
    assert_eq!(outbox.open_count(), 1);
}

#[test]
fn settle_args_round_trip_through_a_claim() {
    let c = claim(1);
    let args: SettleArgs = c.args();
    assert_eq!(args.fragment, 1);
    assert_eq!(args.approvers, c.approvers);
}

#[test]
fn open_authorizations_keep_our_signatures_until_the_settlement_is_done() {
    let chain = funded_chain();
    let mut outbox = outbox();
    let id = outbox.enqueue(authorized(claim(0), &[70])).unwrap();
    let open = outbox.open_authorizations();
    assert_eq!(
        open.len(),
        1,
        "un règlement en attente de signatures reste ouvert"
    );
    assert_eq!(
        open[0].signatures.len(),
        1,
        "la signature déjà reçue est là"
    );
    outbox.enqueue(authorized(claim(0), &[71])).unwrap();
    run(&mut outbox, &chain, 0, 1_000, 4);
    assert_eq!(outbox.status(&id, 0), Some(Status::Confirmed));
    assert!(
        outbox.open_authorizations().is_empty(),
        "réglé : plus rien à rediffuser"
    );
}
