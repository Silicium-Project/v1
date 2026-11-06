/// Réglages de l'outbox.
#[derive(Debug, Clone)]
pub struct OutboxConfig {
    /// Règlements suivis au plus.
    pub max_entries: usize,
    /// Délai après le premier échec ; double à chaque échec consécutif.
    pub backoff_base_ms: u64,
    pub backoff_max_ms: u64,
    /// Au bout de ce temps sans réponse de la chaîne, l'état affiché devient `Unknown`.
    pub unknown_after_ms: u64,
    /// Délai entre deux lectures d'une transaction en attente.
    pub poll_ms: u64,
    /// Soumissions échouées avant d'abandonner (`Rejected`).
    pub max_attempts: u32,
}

impl Default for OutboxConfig {
    fn default() -> Self {
        Self {
            max_entries: 4096,
            backoff_base_ms: 1_000,
            backoff_max_ms: 60_000,
            unknown_after_ms: 120_000,
            poll_ms: 1_000,
            max_attempts: 5,
        }
    }
}

impl OutboxConfig {
    /// Délai d'attente après `failures` échecs consécutifs (≥ 1).
    pub fn backoff(&self, failures: u32) -> u64 {
        let shift = failures.saturating_sub(1).min(20);
        self.backoff_base_ms
            .saturating_mul(1u64 << shift)
            .min(self.backoff_max_ms)
    }
}
