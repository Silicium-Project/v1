//! Règles du mode tolérance, lues du descripteur du workload.
//!
//! Le workload vient du `JobSpec`, signé par le requester : le mode ne peut pas être rétrogradé
//! par un worker ou un vérificateur. Le modèle et le runtime sont figés dans le descripteur.

use sil_crypto::sha256;
use sil_types::Hash32;
use sil_workload_spec::Verification;

/// Hash d'une version de runtime : SHA-256 de son identifiant texte (par ex. `"1.31.1"`).
pub fn runtime_hash(ort_version: &str) -> Hash32 {
    sha256(ort_version.as_bytes())
}

/// Règles de tolérance d'un workload : (modèle, runtime, cosinus minimal en millionièmes).
/// `None` si le workload n'est pas en mode tolérance, ou s'il est inconnu.
pub fn rules_of(workload: &str) -> Option<(Hash32, Hash32, i32)> {
    let spec = sil_workload_spec::find(workload)?;
    match spec.verification {
        Verification::SamplingTolerance {
            cos_min,
            model_sha256,
            ort_version,
            ..
        } => Some((
            hash_from_hex(model_sha256)?,
            runtime_hash(ort_version),
            micro(cos_min),
        )),
        Verification::Recompute | Verification::Sampling { .. } => None,
    }
}

/// Un cosinus en millionièmes. Les descripteurs ne portent que des valeurs entre 0 et 1.
fn micro(value: f64) -> i32 {
    let scaled = (value * 1_000_000.0).round();
    if scaled.is_finite() && (0.0..=1_000_000.0).contains(&scaled) {
        // Borné ci-dessus : la conversion ne déborde pas.
        #[allow(clippy::cast_possible_truncation)]
        let rounded = scaled as i32;
        rounded
    } else {
        0
    }
}

fn hash_from_hex(text: &str) -> Option<Hash32> {
    if text.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(Hash32::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_runtime_hash_is_stable() {
        assert_eq!(runtime_hash("1.31.1"), runtime_hash("1.31.1"));
        assert_ne!(runtime_hash("1.31.1"), runtime_hash("1.23.2"));
    }

    #[test]
    fn the_embedding_descriptor_gives_its_pins() {
        let (model, runtime, cos_min) = rules_of("embedding/e5-small").expect("mode tolérance");
        assert_eq!(model.as_bytes()[0], 0xca);
        assert_eq!(runtime, runtime_hash("1.31.1"));
        assert_eq!(cos_min, 999_000);
    }

    #[test]
    fn exact_and_unknown_workloads_have_no_tolerance_rules() {
        assert!(rules_of("calcul/sha256").is_none());
        assert!(rules_of("rendu/raytracer").is_none());
        assert!(rules_of("ia/llm").is_none());
    }

    #[test]
    fn a_tolerance_attestation_of_another_job_is_refused_by_shape() {
        use crate::EventBody;
        use sil_attest::{ToleranceAttestation, ToleranceBody};
        use sil_crypto::{MasterSeed, Purpose};
        use sil_types::{JobId, TaskRef};

        let identity = MasterSeed::from_bytes([3; 32]).derive(Purpose::Ledger);
        let body = ToleranceBody {
            task: TaskRef::new(JobId::from_bytes([1; 32]), 0),
            attempt: 1,
            result: Hash32::from_bytes([7; 32]),
            model: Hash32::from_bytes([8; 32]),
            runtime: Hash32::from_bytes([9; 32]),
            sample_root: Hash32::from_bytes([4; 32]),
            min_cos_micro: 999_500,
            approved: true,
            evidence: None,
        };
        let attestation = ToleranceAttestation::sign(&identity, body).unwrap();
        let event = EventBody::AttestTolerance { attestation };
        assert!(event.validate_shape(&JobId::from_bytes([1; 32])).is_ok());
        assert!(event.validate_shape(&JobId::from_bytes([2; 32])).is_err());
    }

    #[test]
    fn a_malformed_model_hash_is_refused() {
        assert!(hash_from_hex("abc").is_none());
        assert!(hash_from_hex(&"zz".repeat(32)).is_none());
    }
}
