//! Descripteurs des workloads du réseau.
//!
//! Un workload se décrit une fois, dans `contracts/workloads/*.json`. [`generated`] est produit par
//! `tools/workloads/generate.mjs` : ne pas le modifier à la main. Ce crate ne contient que des
//! données ; le découpage, l'exécution et la vérification restent dans chaque crate `sil-wl-*`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod generated;

pub use generated::*;

/// Type d'unité de travail : ce qu'un fragment rapporte. Un type nouveau demande une règle on-chain
/// (`sil-chain-core::units`) ; un workload de type connu n'en demande aucune.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Une unité = un octet de l'entrée.
    BytesInput,
    /// Une unité = un pixel d'une bande de blocs de lignes.
    PixelsBand,
}

/// Comment un résultat est vérifié (voir `sil_workload_api::VerificationPolicy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verification {
    /// Le vérificateur refait tout le calcul.
    Recompute,
    /// Le vérificateur refait un échantillon de blocs.
    Sampling {
        /// Plus petite fraction de blocs falsifiés qu'on veut détecter.
        min_cheat_fraction: f64,
        /// Probabilité de détection visée pour cette fraction.
        target_detection: f64,
    },
    /// Le vérificateur refait un échantillon d'éléments et compare par cosinus (embedding).
    SamplingTolerance {
        min_cheat_fraction: f64,
        target_detection: f64,
        /// Cosinus minimal entre le vecteur reçu et le recalcul.
        cos_min: f64,
        /// SHA-256 du fichier modèle, figé.
        model_sha256: &'static str,
        /// Version d'ONNX Runtime qui produit les vecteurs de référence.
        ort_version: &'static str,
    },
}

/// Bornes d'exécution d'un fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub wall_ms: u64,
    pub memory_bytes: u64,
    pub output_bytes: u64,
}

/// Intensité relative (0 à 100) que ce workload sollicite sur chaque dimension, jamais son nom
/// (ADR 0013 §3) : permet au réseau d'apprendre la relation ressources → résultat sans nommer de
/// type de tâche. Au moins une dimension est non nulle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resources {
    pub compute: u8,
    pub memory: u8,
    pub network: u8,
    pub storage: u8,
}

/// Descripteur d'un workload (une entrée de `generated.rs`).
///
/// `max_side`, `block_rows` valent 0 quand l'unité ne les utilise pas (`Unit::BytesInput`).
/// `params` : les champs que le site demande en plus du fichier, dans l'ordre de l'en-tête.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    pub id: &'static str,
    pub unit: Unit,
    pub label_key: &'static str,
    pub params: &'static [&'static str],
    pub max_input_bytes: u64,
    pub max_side: u32,
    pub block_rows: u32,
    pub max_fragments: u32,
    pub target_size: u64,
    pub reward_per_unit: u64,
    pub verifier_reward_per_unit: u64,
    pub verification: Verification,
    pub limits: Limits,
    pub deposit_supported: bool,
    pub resources: Resources,
}

/// Le descripteur d'un identifiant, s'il existe.
pub fn find(id: &str) -> Option<&'static Spec> {
    ALL.iter().find(|spec| spec.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_descriptor_is_found_by_its_id() {
        for spec in ALL {
            assert_eq!(find(spec.id), Some(spec));
        }
    }

    #[test]
    fn identifiers_are_unique() {
        for (index, spec) in ALL.iter().enumerate() {
            assert!(ALL[index + 1..].iter().all(|other| other.id != spec.id));
        }
    }

    #[test]
    fn every_descriptor_solicits_at_least_one_resource_dimension() {
        // ADR 0013 §3 : un vecteur de ressources entièrement nul n'apprendrait rien.
        for spec in ALL {
            let r = spec.resources;
            assert!(
                r.compute > 0 || r.memory > 0 || r.network > 0 || r.storage > 0,
                "{} : vecteur de ressources vide",
                spec.id
            );
            for (name, value) in [
                ("compute", r.compute),
                ("memory", r.memory),
                ("network", r.network),
                ("storage", r.storage),
            ] {
                assert!(value <= 100, "{} : {name} dépasse 100", spec.id);
            }
        }
    }

    #[test]
    fn an_unknown_identifier_has_no_descriptor() {
        assert!(find("ia/llm").is_none());
    }

    #[test]
    fn image_units_declare_their_geometry() {
        for spec in ALL {
            match spec.unit {
                Unit::PixelsBand => {
                    assert!(spec.block_rows > 0 && spec.max_side > 0, "{}", spec.id);
                    assert_eq!(spec.params, ["width", "height"], "{}", spec.id);
                }
                Unit::BytesInput => assert!(spec.params.is_empty(), "{}", spec.id),
            }
        }
    }
}
