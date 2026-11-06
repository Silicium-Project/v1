#[derive(Debug, Clone)]
pub struct ReputationConfig {
    /// Temps au bout duquel une observation ne pèse plus que la moitié.
    pub half_life_ms: u64,
    /// Poids maximal d'un succès : un gros contrat ne vaut jamais plus de deux succès ordinaires.
    pub max_success_weight: f64,
    /// Tâches simultanées maximales pour un worker connu, au plus.
    pub max_parallel: u32,
    /// Workers suivis au plus.
    pub max_records: usize,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            half_life_ms: 7 * 24 * 3600 * 1000,
            max_success_weight: 2.0,
            max_parallel: 8,
            max_records: 100_000,
        }
    }
}
