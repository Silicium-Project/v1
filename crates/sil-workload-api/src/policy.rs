/// Bornes d'exécution imposées au sandbox et vérifiées sur le résultat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Durée maximale d'un fragment.
    pub wall_ms: u64,
    /// Mémoire maximale (appliquée par le sandbox quand l'OS le permet).
    pub memory_bytes: u64,
    /// Taille maximale du résultat.
    pub output_bytes: u64,
}

/// Comment un résultat est vérifié par chaque vérificateur retenu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerificationPolicy {
    /// Le vérificateur refait tout le calcul. Avec V vérificateurs : V calculs indépendants
    /// (c'est la redondance).
    Recompute,
    /// Le vérificateur refait un échantillon de blocs, dimensionné par
    /// `sil_sampling::samples_needed(blocs, min_cheat_fraction, target_detection)`.
    Sampling {
        /// Plus petite fraction de blocs falsifiés qu'on veut détecter.
        min_cheat_fraction: f64,
        /// Probabilité de détection visée pour cette fraction.
        target_detection: f64,
    },
    /// Le vérificateur refait un échantillon d'éléments et compare chacun par cosinus (embedding).
    /// Un élément sous cos_min, ou un seul élément ne passant pas, fait rejeter le fragment.
    SamplingTolerance {
        min_cheat_fraction: f64,
        target_detection: f64,
        cos_min: f64,
        model_sha256: &'static str,
        ort_version: &'static str,
    },
}
