use super::support::*;
use crate::{GuardError, check_escrow, terms_of};
use sil_chain_core::Escrow;
use sil_ledger::JobSpec;
use sil_types::Hash32;

fn spec() -> JobSpec {
    JobSpec {
        version: 2,
        requester: ident(100).public(),
        nonce: 1,
        workload: "calcul/sha256".into(),
        input: Hash32::from_bytes([1; 32]),
        input_size: 20,
        width: 0,
        height: 0,
        fragments: 2,
        verifications_required: 2,
        lease_ms: 60_000,
        max_attempts: 3,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: 1_000_000,
    }
}

/// Le dépôt qui couvre tous les fragments de la spec.
fn budget(spec: &JobSpec) -> u64 {
    crate::terms_of(spec).max_payout().unwrap()
}

fn escrow_for(spec: &JobSpec, deposit: u64) -> Escrow {
    Escrow::open(
        spec.job_id().unwrap(),
        terms_of(spec),
        deposit,
        &spec.requester,
    )
    .unwrap()
}

#[test]
fn a_matching_funded_escrow_is_accepted() {
    let spec = spec();
    assert_eq!(
        check_escrow(&spec, &escrow_for(&spec, budget(&spec)), 0, 100_000),
        Ok(())
    );
}

#[test]
fn an_escrow_of_another_job_or_with_other_terms_is_refused() {
    let spec = spec();
    let mut other_spec = spec.clone();
    other_spec.nonce = 2;
    assert_eq!(
        check_escrow(&spec, &escrow_for(&other_spec, budget(&other_spec)), 0, 0),
        Err(GuardError::WrongJob)
    );

    let mut cheaper = escrow_for(&spec, budget(&spec));
    cheaper.terms.reward_per_unit = 1;
    assert_eq!(
        check_escrow(&spec, &cheaper, 0, 0),
        Err(GuardError::TermsMismatch("tarif par unité"))
    );

    // Une entrée de taille différente change les unités, donc les montants : refusé.
    let mut resized = escrow_for(&spec, budget(&spec));
    resized.terms.input_size = 1;
    assert_eq!(
        check_escrow(&spec, &resized, 0, 0),
        Err(GuardError::TermsMismatch("taille de l'entrée"))
    );
}

#[test]
fn closed_or_too_late_escrows_are_refused() {
    let spec = spec();
    let mut closed = escrow_for(&spec, budget(&spec));
    closed
        .refund(&spec.requester, 1_000_000 + sil_chain_core::REFUND_GRACE_MS)
        .unwrap();
    assert_eq!(check_escrow(&spec, &closed, 0, 0), Err(GuardError::Closed));

    // Un fragment déjà payé réduit le besoin de fonds restant.
    let mut partly = escrow_for(&spec, budget(&spec));
    let signers = [ident(70).public(), ident(71).public()];
    partly
        .settle(
            &claim(0).args(),
            &signers,
            &sil_types::PublicKey::from_bytes([0xEE; 32]),
            &[Some(sil_chain_core::reputation::VERIFIER_START); 2],
        )
        .unwrap();
    assert_eq!(
        check_escrow(&spec, &partly, 0, 0),
        Ok(()),
        "il reste un fragment à payer"
    );

    let late = escrow_for(&spec, budget(&spec));
    assert_eq!(
        check_escrow(&spec, &late, 950_000, 100_000),
        Err(GuardError::TooLate)
    );
}
