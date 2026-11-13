// GÉNÉRÉ par tools/workloads/generate.mjs depuis contracts/workloads/*.json. Ne pas modifier à la main.

use crate::{Limits, Resources, Spec, Unit, Verification};

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
    resources: Resources {
        compute: 70,
        memory: 10,
        network: 20,
        storage: 20,
    },
};

/// `embedding/e5-small` : Une unité = un octet de l'entrée. L'entrée est un lot de textes (un texte par ligne, découpé en chunks de 512 tokens au plus). Chaque chunk reçoit son vecteur à 384 dimensions (multilingual-e5-small, fp32, préfixe « passage: »). Vérification par tolérance : un vérificateur recalcule un échantillon de chunks et compare chaque vecteur par cosinus.
pub const EMBEDDING_E5_SMALL: Spec = Spec {
    id: "embedding/e5-small",
    unit: Unit::BytesInput,
    label_key: "submit.workloadEmbedding",
    params: &[],
    max_input_bytes: 8388608,
    max_side: 0,
    block_rows: 0,
    max_fragments: 16,
    target_size: 262144,
    reward_per_unit: 100,
    verifier_reward_per_unit: 10,
    verification: Verification::SamplingTolerance {
        min_cheat_fraction: 0.1,
        target_detection: 0.97,
        cos_min: 0.999,
        model_sha256: "ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665",
        ort_version: "1.31.1",
    },
    limits: Limits {
        wall_ms: 300000,
        memory_bytes: 1073741824,
        output_bytes: 67108864,
    },
    deposit_supported: true,
    resources: Resources {
        compute: 80,
        memory: 60,
        network: 20,
        storage: 40,
    },
};

/// `image/batch` : Une unité = un octet de l'entrée. L'entrée est un lot d'images (conteneur SILBATCH) : chaque image est redimensionnée et réencodée (PNG ou JPEG). Chaque fragment est une plage d'images, dont le résultat est publié ; un vérificateur refait le traitement de la plage.
pub const IMAGE_BATCH: Spec = Spec {
    id: "image/batch",
    unit: Unit::BytesInput,
    label_key: "submit.workloadImage",
    params: &[],
    max_input_bytes: 8388608,
    max_side: 0,
    block_rows: 0,
    max_fragments: 16,
    target_size: 262144,
    reward_per_unit: 100,
    verifier_reward_per_unit: 10,
    verification: Verification::Sampling {
        min_cheat_fraction: 0.2,
        target_detection: 0.99,
    },
    limits: Limits {
        wall_ms: 120000,
        memory_bytes: 536870912,
        output_bytes: 67108864,
    },
    deposit_supported: true,
    resources: Resources {
        compute: 60,
        memory: 30,
        network: 20,
        storage: 30,
    },
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
    resources: Resources {
        compute: 90,
        memory: 40,
        network: 10,
        storage: 30,
    },
};

/// Tous les descripteurs, triés par identifiant.
pub const ALL: &[Spec] = &[CALCUL_SHA256, EMBEDDING_E5_SMALL, IMAGE_BATCH, RENDU_RAYTRACER];
