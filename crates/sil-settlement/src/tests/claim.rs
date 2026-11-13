use super::support::*;
use crate::*;
use sil_ledger::Finalized;
use sil_types::Hash32;

fn finalized(approvers: Vec<u8>) -> Finalized {
    Finalized {
        attempt: 2,
        result: Hash32::from_bytes([7; 32]),
        claimant: key(60),
        approvers: approvers.into_iter().map(key).collect(),
    }
}

#[test]
fn a_claim_takes_exactly_the_required_approvers_in_a_canonical_order() {
    let f = finalized(vec![72, 70, 71, 70]);
    let claim = SettlementClaim::from_finalized(&job(), 0, &f, 3).unwrap();
    assert_eq!(claim.approvers, vec![key(70), key(71), key(72)]);
    // Même résultat quel que soit l'ordre vu par le nœud : même identifiant partout.
    let other =
        SettlementClaim::from_finalized(&job(), 0, &finalized(vec![71, 72, 70]), 3).unwrap();
    assert_eq!(claim.id(), other.id());
    assert!(SettlementClaim::from_finalized(&job(), 0, &finalized(vec![70, 71]), 3).is_none());
}

#[test]
fn the_identifier_changes_with_any_parameter() {
    let base = claim(0);
    let mut other = base.clone();
    other.result = Hash32::from_bytes([8; 32]);
    assert_ne!(base.id(), other.id());
    assert_ne!(base.id(), claim(1).id());
}

#[test]
fn only_valid_distinct_approver_signatures_count() {
    let c = claim(0);
    let mut auth = Authorization::new(c.clone());
    let signer = ident(70);
    assert!(auth.add(signer.public(), c.sign(&signer)).unwrap());
    assert!(
        !auth.add(signer.public(), c.sign(&signer)).unwrap(),
        "doublon"
    );

    // Signature d'un autre règlement : refusée.
    let wrong = claim(1).sign(&ident(71));
    assert!(matches!(
        auth.add(ident(71).public(), wrong),
        Err(SettlementError::BadSignature)
    ));

    // Une signature valide d'un non-approbateur existe mais ne compte pas.
    let outsider = ident(90);
    assert!(auth.add(outsider.public(), c.sign(&outsider)).unwrap());
    assert_eq!(auth.approver_signers(), vec![signer.public()]);
}

#[test]
fn a_claim_is_compared_with_what_the_chain_recorded() {
    let c = claim(0);
    let mut settled = sil_chain_core::SettledFragment {
        attempt: c.attempt,
        result: c.result,
        claimant: c.claimant,
        approvers: c.approvers.clone(),
        held: c.approvers.iter().map(|_| sil_chain_core::HeldEntry::default()).collect(),
    };
    assert!(c.matches(&settled));
    settled.result = Hash32::from_bytes([9; 32]);
    assert!(!c.matches(&settled));
}

#[test]
fn the_signed_message_has_a_fixed_layout_a_program_can_rebuild() {
    let c = claim(3);
    let bytes = c.bytes();
    assert_eq!(bytes.len(), 32 + 4 + 4 + 32 + 32);
    assert_eq!(&bytes[..32], c.job.as_bytes());
    assert_eq!(&bytes[32..36], &3u32.to_le_bytes());
    assert_eq!(&bytes[36..40], &1u32.to_le_bytes());
    assert_eq!(&bytes[40..72], c.result.as_bytes());
    assert_eq!(&bytes[72..104], c.claimant.as_bytes());
}
