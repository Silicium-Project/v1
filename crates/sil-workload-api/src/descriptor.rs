//! Limites et politique de vérification d'un workload, lues depuis son descripteur
//! (`contracts/workloads`). Un workload ne redéclare pas ces valeurs : il les prend ici.

use crate::{Limits, VerificationPolicy};
use sil_workload_spec::{Spec, Verification};

/// Les bornes d'exécution du descripteur.
pub fn limits_of(spec: &Spec) -> Limits {
    Limits {
        wall_ms: spec.limits.wall_ms,
        memory_bytes: spec.limits.memory_bytes,
        output_bytes: spec.limits.output_bytes,
    }
}

/// La politique de vérification du descripteur.
pub fn policy_of(spec: &Spec) -> VerificationPolicy {
    match spec.verification {
        Verification::Recompute => VerificationPolicy::Recompute,
        Verification::Sampling {
            min_cheat_fraction,
            target_detection,
        } => VerificationPolicy::Sampling {
            min_cheat_fraction,
            target_detection,
        },
        Verification::SamplingTolerance {
            min_cheat_fraction,
            target_detection,
            cos_min,
            model_sha256,
            ort_version,
        } => VerificationPolicy::SamplingTolerance {
            min_cheat_fraction,
            target_detection,
            cos_min,
            model_sha256,
            ort_version,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_workload_spec::{CALCUL_SHA256, RENDU_RAYTRACER};

    #[test]
    fn sha256_is_verified_by_recomputation() {
        assert_eq!(policy_of(&CALCUL_SHA256), VerificationPolicy::Recompute);
    }

    #[test]
    fn render_is_verified_by_sampling_with_the_descriptor_rates() {
        assert_eq!(
            policy_of(&RENDU_RAYTRACER),
            VerificationPolicy::Sampling {
                min_cheat_fraction: 0.2,
                target_detection: 0.99,
            }
        );
    }

    #[test]
    fn embedding_is_verified_by_tolerance_with_the_descriptor_pins() {
        let policy = policy_of(&sil_workload_spec::EMBEDDING_E5_SMALL);
        assert!(matches!(
            policy,
            VerificationPolicy::SamplingTolerance { cos_min, ort_version, .. }
                if cos_min == 0.999 && ort_version == "1.31.1"
        ));
    }

    #[test]
    fn limits_come_from_the_descriptor() {
        assert_eq!(limits_of(&CALCUL_SHA256).wall_ms, 120_000);
        assert_eq!(limits_of(&RENDU_RAYTRACER).output_bytes, 256 << 20);
    }
}
