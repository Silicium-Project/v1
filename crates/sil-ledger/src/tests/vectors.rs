//! Vecteurs partagés avec le site (`apps/web/src/signing/`). Ils figent les octets exacts
//! qu'un wallet doit signer pour publier un job, et l'événement final.
//!
//! Régénérer après un changement voulu du format :
//! `SIL_REGENERATE_VECTORS=1 cargo test -p sil-ledger vectors`.

use crate::wallet_text::spec_text;
use crate::{Event, EventBody, Header, JobSpec, NO_PREVIOUS, SignedJobSpec};
use serde_json::{Value, json};
use sil_crypto::{MasterSeed, Purpose, SigningIdentity};
use sil_types::{Hash32, PublicKey};

const VECTOR_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/vectors/job-published.json"
);

fn identity(seed: u8) -> SigningIdentity {
    MasterSeed::from_bytes([seed; 32]).derive(Purpose::Ledger)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sample_spec(requester: &SigningIdentity, committee: &[PublicKey]) -> JobSpec {
    JobSpec {
        version: 1,
        requester: requester.public(),
        nonce: 1,
        workload: "calcul/sha256".into(),
        input: Hash32::from_bytes([1; 32]),
        input_size: 100,
        width: 0,
        height: 0,
        fragments: 2,
        committee: committee.to_vec(),
        threshold: 2,
        verifications_required: 1,
        lease_ms: 60_000,
        max_attempts: 3,
        reward_per_unit: 10,
        verifier_reward_per_unit: 1,
        expires_at_ms: 1_900_000_000_000,
    }
}

/// Calcule tous les champs du vecteur. Rien d'aléatoire : signatures Ed25519 déterministes.
fn compute() -> Value {
    let requester = identity(7);
    let committee: Vec<PublicKey> = [8u8, 9, 10]
        .into_iter()
        .map(|s| identity(s).public())
        .collect();
    let spec = sample_spec(&requester, &committee);
    let spec_cbor = spec.bytes().expect("spec encodes");
    let spec_message = spec_text(&spec).into_bytes();
    let spec_signature = requester.sign_raw(&spec_message);
    let job = spec.job_id().expect("job id");
    let signed = SignedJobSpec::sign(&requester, spec.clone()).expect("spec signs");

    let header = Header {
        job,
        sequence: 1,
        previous: NO_PREVIOUS,
        issued_at_ms: 1_800_000_000_000,
    };
    let body = EventBody::JobPublished { spec: signed };
    let event = Event::sign(&requester, header, body).expect("event signs");
    let event_core = event.core_bytes().expect("core encodes");
    let event_message = event
        .signing_text()
        .expect("JobPublished is signed as text")
        .into_bytes();
    let event_cbor = event.to_bytes().expect("event encodes");

    json!({
        "requester": hex(requester.public().as_bytes()),
        "committee": committee.iter().map(|k| hex(k.as_bytes())).collect::<Vec<_>>(),
        "threshold": 2,
        "spec_cbor": hex(&spec_cbor),
        "spec_message": hex(&spec_message),
        "spec_signature": hex(spec_signature.as_bytes()),
        "job_id": hex(job.as_bytes()),
        "event_core": hex(&event_core),
        "event_message": hex(&event_message),
        "event_signature": hex(event.signature.as_bytes()),
        "event_cbor": hex(&event_cbor),
    })
}

#[test]
fn vectors_match_the_contract_file() {
    let computed = compute();
    if std::env::var_os("SIL_REGENERATE_VECTORS").is_some() {
        let text = serde_json::to_string_pretty(&computed).expect("json");
        std::fs::write(VECTOR_FILE, format!("{text}\n")).expect("write vectors");
        return;
    }
    let stored: Value =
        serde_json::from_str(&std::fs::read_to_string(VECTOR_FILE).expect("vector file"))
            .expect("json");
    assert_eq!(
        stored, computed,
        "the signing format changed: regenerate and update the web side"
    );
}

#[test]
fn the_vector_spec_verifies_with_the_public_api() {
    let requester = identity(7);
    let committee: Vec<PublicKey> = [8u8, 9, 10]
        .into_iter()
        .map(|s| identity(s).public())
        .collect();
    let signed =
        SignedJobSpec::sign(&requester, sample_spec(&requester, &committee)).expect("spec signs");
    assert!(signed.verify().is_ok());
}

#[test]
fn the_signed_texts_are_ascii_and_name_their_domain() {
    let requester = identity(7);
    let committee: Vec<PublicKey> = [8u8, 9, 10]
        .into_iter()
        .map(|s| identity(s).public())
        .collect();
    let spec = sample_spec(&requester, &committee);
    let text = spec_text(&spec);
    assert!(text.is_ascii(), "the wallet must show plain text");
    assert!(text.starts_with("Silicium JobSpec v1\n"));
    assert!(text.contains("\nWorkload: calcul/sha256\n"));
    let signed = SignedJobSpec::sign(&requester, spec).expect("spec signs");
    let event = Event::sign(
        &requester,
        Header {
            job: signed.verify().expect("job id"),
            sequence: 1,
            previous: NO_PREVIOUS,
            issued_at_ms: 1,
        },
        EventBody::JobPublished { spec: signed },
    )
    .expect("event signs");
    let event_text = event.signing_text().expect("text");
    assert!(event_text.is_ascii());
    assert!(event_text.starts_with("Silicium Event v1\n"));
    assert!(event.verify_signature().is_ok());
}
