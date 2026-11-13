//! Le verdict signé se reconstruit à l'identique on-chain (ADR 0012) : même octets que
//! `sil_chain_core::wire::attestation_frame`, préfixe de domaine compris.

use crate::attestation::verdict_message;
use sil_chain_core::wire::{ATTESTATION_DOMAIN, attestation_frame};
use sil_types::{Hash32, JobId, TaskRef};

#[test]
fn the_tolerance_verdict_bytes_are_the_chain_frame_after_the_domain() {
    use crate::tolerance::{ToleranceBody, tolerance_message};
    use sil_chain_core::wire::{TOLERANCE_DOMAIN, tolerance_frame};
    let body = ToleranceBody {
        task: TaskRef::new(JobId::from_bytes([1; 32]), 3),
        attempt: 2,
        result: Hash32::from_bytes([7; 32]),
        model: Hash32::from_bytes([11; 32]),
        runtime: Hash32::from_bytes([12; 32]),
        sample_root: Hash32::from_bytes([13; 32]),
        min_cos_micro: 912_345,
        approved: false,
        evidence: Some(Hash32::from_bytes([5; 32])),
    };
    let ours = tolerance_message(&body);
    let frame = tolerance_frame(
        &body.task.job,
        3,
        2,
        &body.result,
        &body.model,
        &body.runtime,
        &body.sample_root,
        body.min_cos_micro,
        false,
        body.evidence.as_ref(),
    );
    assert!(frame.starts_with(TOLERANCE_DOMAIN));
    assert_eq!(&frame[TOLERANCE_DOMAIN.len()..], ours.as_slice());
    assert_eq!(ours.len(), 206);
}

#[test]
fn the_verdict_bytes_are_the_chain_frame_after_the_domain() {
    let task = TaskRef::new(JobId::from_bytes([1; 32]), 3);
    let result = Hash32::from_bytes([7; 32]);
    let evidence = Hash32::from_bytes([5; 32]);
    for (approved, proof) in [(true, None), (false, Some(&evidence))] {
        let ours = verdict_message(&task, 2, &result, approved, proof);
        let frame = attestation_frame(&task.job, 3, 2, &result, approved, proof);
        assert!(frame.starts_with(ATTESTATION_DOMAIN));
        assert_eq!(&frame[ATTESTATION_DOMAIN.len()..], ours.as_slice());
        assert_eq!(ours.len(), 106);
    }
}
