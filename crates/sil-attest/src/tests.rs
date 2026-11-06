use crate::{AttestError, Attestation, AttestationBody, CertRules, Certificate};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_types::{Hash32, JobId, TaskRef};

fn id(n: u8) -> SigningIdentity {
    MasterSeed::from_bytes([n; 32]).derive(Purpose::Ledger)
}

fn task() -> TaskRef {
    TaskRef::new(JobId::from_bytes([1; 32]), 0)
}

fn result() -> Hash32 {
    Hash32::from_bytes([2; 32])
}

fn body(approved: bool) -> AttestationBody {
    AttestationBody {
        task: task(),
        attempt: 1,
        result: result(),
        approved,
        evidence: (!approved).then(|| Hash32::from_bytes([3; 32])),
    }
}

fn approve(who: &SigningIdentity) -> Attestation {
    Attestation::sign(who, body(true)).unwrap()
}

fn cert(approvals: Vec<Attestation>) -> Certificate {
    Certificate {
        task: task(),
        attempt: 1,
        result: result(),
        approvals,
    }
}

fn anyone(_: &sil_types::PublicKey) -> bool {
    true
}

fn rules<'a>(required: u32, excluded: &'a [sil_types::PublicKey]) -> CertRules<'a> {
    CertRules {
        required,
        excluded,
        eligible: &anyone,
    }
}

#[test]
fn a_rejection_needs_evidence_and_an_approval_forbids_it() {
    let mut rejection = body(false);
    rejection.evidence = None;
    assert_eq!(
        Attestation::sign(&id(1), rejection),
        Err(AttestError::MissingEvidence)
    );
    let mut approval = body(true);
    approval.evidence = Some(Hash32::from_bytes([9; 32]));
    assert_eq!(
        Attestation::sign(&id(1), approval),
        Err(AttestError::UnexpectedEvidence)
    );
}

#[test]
fn tampering_breaks_the_signature() {
    let mut attestation = approve(&id(1));
    assert!(attestation.verify().is_ok());
    attestation.body.attempt = 2;
    assert_eq!(attestation.verify(), Err(AttestError::BadSignature));
}

#[test]
fn a_valid_quorum_passes() {
    let certificate = cert(vec![approve(&id(1)), approve(&id(2))]);
    assert!(certificate.verify(&rules(2, &[])).is_ok());
}

#[test]
fn too_few_duplicates_and_excluded_verifiers_fail() {
    let one = cert(vec![approve(&id(1))]);
    assert!(matches!(
        one.verify(&rules(2, &[])),
        Err(AttestError::TooFew { have: 1, need: 2 })
    ));
    let twice = cert(vec![approve(&id(1)), approve(&id(1))]);
    assert_eq!(
        twice.verify(&rules(1, &[])),
        Err(AttestError::DuplicateVerifier)
    );
    let claimant = [id(1).public()];
    assert_eq!(
        one.verify(&rules(1, &claimant)),
        Err(AttestError::ExcludedVerifier)
    );
}

#[test]
fn rejections_and_other_results_cannot_be_smuggled_in() {
    let rejection = Attestation::sign(&id(1), body(false)).unwrap();
    assert_eq!(
        cert(vec![rejection]).verify(&rules(1, &[])),
        Err(AttestError::NotApproval)
    );
    let mut other = body(true);
    other.result = Hash32::from_bytes([8; 32]);
    let foreign = Attestation::sign(&id(2), other).unwrap();
    assert!(matches!(
        cert(vec![foreign]).verify(&rules(1, &[])),
        Err(AttestError::Mismatch(_))
    ));
}

#[test]
fn eligibility_and_requirement_are_enforced() {
    let certificate = cert(vec![approve(&id(1))]);
    let nobody = |_: &sil_types::PublicKey| false;
    let strict = CertRules {
        required: 1,
        excluded: &[],
        eligible: &nobody,
    };
    assert_eq!(
        certificate.verify(&strict),
        Err(AttestError::IneligibleVerifier)
    );
    assert_eq!(
        certificate.verify(&rules(0, &[])),
        Err(AttestError::InvalidRequirement)
    );
}
