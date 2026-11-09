// GÉNÉRÉ par tools/workloads/generate.mjs depuis contracts/workloads/*.json. Ne pas modifier à la main.

use crate::{Limits, Spec, Unit, Verification};

/// `calcul/sha256` : Une unité = un octet de l'entrée. Chaque fragment est une plage d'octets dont le SHA-256 est publié ; un vérificateur refait le hash de la plage.
pub const CALCUL_SHA256: Spec = Spec {
    id: "calcul/sha256",
    unit: Unit::BytesInput,
    label_key: "submit.workloadSha256",
    params: &[],
    max_input_bytes: 8388608,
    max_side: 0,
    block_rows: 0,
    max_fragments: 16,
    target_size: 262144,
    reward_per_unit: 100,
    verifier_reward_per_unit: 10,
    verification: Verification::Recompute,
    limits: Limits {
        wall_ms: 120000,
        memory_bytes: 67108864,
        output_bytes: 32,
    },
    deposit_supported: true,
};

/// `rendu/raytracer` : Une unité = un pixel de la bande du fragment. L'entrée est une scène .cfg, rendue par le moteur raytracing déterministe (sil-raytracer) ; un vérificateur recalcule un échantillon de blocs de 8 lignes.
pub const RENDU_RAYTRACER: Spec = Spec {
    id: "rendu/raytracer",
    unit: Unit::PixelsBand,
    label_key: "submit.workloadRender",
    params: &["width", "height"],
    max_input_bytes: 8388608,
    max_side: 4096,
    block_rows: 8,
    max_fragments: 16,
    target_size: 32,
    reward_per_unit: 1,
    verifier_reward_per_unit: 1,
    verification: Verification::Sampling {
        min_cheat_fraction: 0.2,
        target_detection: 0.99,
    },
    limits: Limits {
        wall_ms: 600000,
        memory_bytes: 1073741824,
        output_bytes: 268435456,
    },
    deposit_supported: true,
};

/// Tous les descripteurs, triés par identifiant.
pub const ALL: &[Spec] = &[CALCUL_SHA256, RENDU_RAYTRACER];
