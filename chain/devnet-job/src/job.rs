//! Spec du job et événement `JobPublished`, signés par le requester comme le fait le site avec
//! son wallet. Les mêmes règles que `chain/e2e` : fragments, seuil et tarifs.

use sil_content::Manifest;
use sil_crypto::SigningIdentity;
use sil_ledger::{Event, EventBody, Header, JobSpec, NO_PREVIOUS, SignedJobSpec};

/// Version de la spec acceptée par le journal (`sil_ledger::spec::SPEC_VERSION`, module privé).
const SPEC_VERSION: u8 = 2;
use sil_types::{Hash32, JobId};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Built {
    pub spec: JobSpec,
    pub job: JobId,
    /// Événement `JobPublished` encodé (CBOR canonique), à envoyer tel quel.
    pub event: Vec<u8>,
}

/// Tarifs par unité (worker, vérificateur) : ceux du descripteur du workload (`contracts/workloads`).
pub fn rewards(workload: &str) -> (u64, u64) {
    if workload == "rendu/raytracer" {
        (1, 1)
    } else {
        (100, 10)
    }
}

/// Entrée d'un job de rendu : largeur puis hauteur (u32 little-endian), puis la scène.
pub fn image_payload(width: u32, height: u32, scene: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8 + scene.len());
    payload.extend_from_slice(&width.to_le_bytes());
    payload.extend_from_slice(&height.to_le_bytes());
    payload.extend_from_slice(scene);
    payload
}

/// Construit la spec pour l'entrée `payload` et la signe. Le requester est l'identité elle-même.
pub fn build(
    identity: &SigningIdentity,
    payload: &[u8],
    fragments: u32,
    workload: &str,
    (width, height): (u32, u32),
) -> Result<Built, String> {
    let now = now_ms();
    let (reward, verifier_reward) = rewards(workload);
    let spec = JobSpec {
        version: SPEC_VERSION,
        requester: identity.public(),
        nonce: now,
        workload: workload.into(),
        input: Hash32::from_bytes(*Manifest::of_bytes(payload).id().as_bytes()),
        input_size: payload.len() as u64,
        width,
        height,
        fragments,
        verifications_required: 1,
        lease_ms: 60_000,
        max_attempts: 3,
        reward_per_unit: reward,
        verifier_reward_per_unit: verifier_reward,
        expires_at_ms: now + 3_600_000,
    };
    let signed = SignedJobSpec::sign(identity, spec.clone()).map_err(|e| e.to_string())?;
    let job = signed.verify().map_err(|e| e.to_string())?;
    let header = Header {
        job,
        sequence: 1,
        previous: NO_PREVIOUS,
        issued_at_ms: now,
    };
    let event = Event::sign(identity, header, EventBody::JobPublished { spec: signed })
        .map_err(|e| e.to_string())?;
    Ok(Built {
        spec,
        job,
        event: event.to_bytes().map_err(|e| e.to_string())?,
    })
}

/// Entrée de test déterministe : même octets à chaque exécution.
pub fn input_bytes(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i.wrapping_mul(31) ^ (i >> 5)) as u8)
        .collect()
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}
