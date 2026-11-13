use super::support::*;
use crate::{PenaltyBox, PenaltyClaim, PenaltyPhase};
use sil_chain_core::PenaltyArgs;
use sil_chain_core::wire::attestation_frame;
use sil_types::Hash32;

/// Verdict de `verifier` sur le fragment 0, tentative 1, signé comme `sil-attest` le fait.
fn verdict(verifier: u8, approved: bool, result: Hash32, evidence: Option<Hash32>) -> PenaltyClaim {
    let frame = attestation_frame(&job(), 0, 1, &result, approved, evidence.as_ref());
    PenaltyClaim {
        job: job(),
        requester: ident(100).public(),
        approvers: vec![ident(70).public(), ident(71).public(), ident(72).public()],
        penalty: PenaltyArgs {
            fragment: 0,
            attempt: 1,
            result,
            approved,
            evidence,
            tolerance: None,
            verifier: ident(verifier).public(),
        },
        signature: ident(verifier).sign_raw(&frame),
    }
}

/// Verdict tolérance : signé sous `Domain::AttestationTolerance`, comme `sil-attest` le fait.
fn tolerance_verdict(verifier: u8, result: Hash32, approved: bool) -> PenaltyClaim {
    use sil_chain_core::ToleranceProof;
    use sil_chain_core::wire::tolerance_frame;
    let proof = ToleranceProof {
        model: Hash32::from_bytes([11; 32]),
        runtime: Hash32::from_bytes([12; 32]),
        sample_root: Hash32::from_bytes([13; 32]),
        min_cos_micro: 900_000,
    };
    let evidence = (!approved).then(|| Hash32::from_bytes([5; 32]));
    let frame = tolerance_frame(
        &job(),
        0,
        1,
        &result,
        &proof.model,
        &proof.runtime,
        &proof.sample_root,
        proof.min_cos_micro,
        approved,
        evidence.as_ref(),
    );
    PenaltyClaim {
        job: job(),
        requester: ident(100).public(),
        approvers: vec![ident(70).public(), ident(71).public(), ident(72).public()],
        penalty: PenaltyArgs {
            fragment: 0,
            attempt: 1,
            result,
            approved,
            evidence,
            tolerance: Some(proof),
            verifier: ident(verifier).public(),
        },
        signature: ident(verifier).sign_raw(&frame),
    }
}

#[test]
fn a_tolerance_verdict_against_the_majority_is_a_valid_penalty() {
    let claim = tolerance_verdict(80, Hash32::from_bytes([9; 32]), true);
    assert!(claim.is_signed(), "signé sous le domaine tolérance");
    let mut queue = PenaltyBox::new();
    assert!(queue.enqueue(claim).is_ok());
}

#[test]
fn a_tolerance_verdict_signed_as_exact_is_refused() {
    // Même contenu, mauvais domaine : une attestation exacte ne se rejoue pas en tolérance.
    let mut claim = tolerance_verdict(80, Hash32::from_bytes([9; 32]), true);
    claim.penalty.tolerance = None;
    assert!(!claim.is_signed());
}

#[test]
fn a_tolerance_rejection_of_the_settled_result_is_confirmed_on_chain() {
    let chain = funded_chain();
    let mut outbox = outbox();
    outbox
        .enqueue(authorized(claim(0), &[70, 71, 72]))
        .unwrap();
    run(&mut outbox, &chain, 0, 1_000, 4);
    let mut queue = PenaltyBox::new();
    let id = queue
        .enqueue(tolerance_verdict(80, Hash32::from_bytes([7; 32]), false))
        .unwrap();
    for _ in 0..2 {
        queue.advance(&chain);
    }
    assert_eq!(queue.phase(&id), Some(&PenaltyPhase::Confirmed));
}

/// Approuve un autre résultat : contraire à la majorité qui a réglé `[7; 32]`.
fn approves_other(verifier: u8) -> PenaltyClaim {
    verdict(verifier, true, Hash32::from_bytes([9; 32]), None)
}

#[test]
fn a_penalty_without_a_valid_verdict_signature_is_refused() {
    let mut bad = approves_other(80);
    bad.signature = ident(81).sign_raw(&attestation_frame(
        &job(),
        0,
        1,
        &Hash32::from_bytes([9; 32]),
        true,
        None,
    ));
    assert!(PenaltyBox::new().enqueue(bad).is_err());
}

#[test]
fn the_same_proof_is_queued_once() {
    let mut queue = PenaltyBox::new();
    let first = queue.enqueue(approves_other(80)).unwrap();
    let again = queue.enqueue(approves_other(80)).unwrap();
    assert_eq!(first, again);
    assert_eq!(queue.len(), 1);
}

#[test]
fn a_penalty_is_sent_then_confirmed_on_a_settled_fragment() {
    let chain = funded_chain();
    let mut outbox = outbox();
    outbox
        .enqueue(authorized(claim(0), &[70, 71, 72]))
        .unwrap();
    run(&mut outbox, &chain, 0, 1_000, 4);

    let mut queue = PenaltyBox::new();
    let id = queue.enqueue(approves_other(80)).unwrap();
    queue.advance(&chain);
    assert!(matches!(queue.phase(&id), Some(PenaltyPhase::Submitted { .. })));
    queue.advance(&chain);
    assert_eq!(queue.phase(&id), Some(&PenaltyPhase::Confirmed));
}

#[test]
fn a_second_penalty_for_the_same_verifier_is_refused_for_good() {
    let chain = funded_chain();
    let mut outbox = outbox();
    outbox
        .enqueue(authorized(claim(0), &[70, 71, 72]))
        .unwrap();
    run(&mut outbox, &chain, 0, 1_000, 4);

    let mut queue = PenaltyBox::new();
    let first = queue.enqueue(approves_other(80)).unwrap();
    // Même vérificateur, autre verdict contraire signé : le programme refuse « déjà pénalisé ».
    let second = queue
        .enqueue(verdict(80, false, Hash32::from_bytes([7; 32]), Some(Hash32::from_bytes([5; 32]))))
        .unwrap();
    for _ in 0..3 {
        queue.advance(&chain);
    }
    // Une seule des deux passe ; l'autre est refusée « déjà pénalisé ». L'ordre dépend des ids.
    let phases = [queue.phase(&first).cloned(), queue.phase(&second).cloned()];
    let confirmed = phases
        .iter()
        .filter(|p| **p == Some(PenaltyPhase::Confirmed))
        .count();
    let refused = phases
        .iter()
        .filter(|p| matches!(p, Some(PenaltyPhase::Rejected { reason }) if reason.contains("déjà pénalisé")))
        .count();
    assert_eq!((confirmed, refused), (1, 1), "{phases:?}");
}

#[test]
fn the_chain_refuses_a_penalty_on_an_unsettled_fragment() {
    // Le nœud n'envoie qu'après confirmation du règlement (finalize_loop) ; ici la boîte seule.
    let chain = funded_chain();
    let mut queue = PenaltyBox::new();
    let id = queue.enqueue(approves_other(80)).unwrap();
    queue.advance(&chain);
    assert!(matches!(queue.phase(&id), Some(PenaltyPhase::Rejected { .. })));
}
