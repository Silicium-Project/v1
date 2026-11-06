/// Plafonds du journal. Chaque dépassement est refusé avec une erreur typée, jamais ignoré.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerLimits {
    /// Jobs suivis simultanément.
    pub max_jobs: usize,
    /// Jobs publiés par un même requester.
    pub max_jobs_per_requester: usize,
    /// Événements stockés par job.
    pub max_events_per_job: usize,
    /// Événements stockés par (auteur, job).
    pub max_events_per_author_job: usize,
    /// Octets d'événements stockés par job.
    pub max_bytes_per_job: usize,
    /// Taille encodée maximale d'un événement.
    pub max_event_bytes: usize,
    /// Événements en attente d'un prédécesseur, par (auteur, job).
    pub max_pending_per_author_job: usize,
    /// Écart de séquence toléré avant refus immédiat.
    pub max_sequence_gap: u64,
}

impl Default for LedgerLimits {
    fn default() -> Self {
        Self {
            max_jobs: 4096,
            max_jobs_per_requester: 64,
            max_events_per_job: 4096,
            max_events_per_author_job: 256,
            max_bytes_per_job: 4 * 1024 * 1024,
            max_event_bytes: 32 * 1024,
            max_pending_per_author_job: 16,
            max_sequence_gap: 32,
        }
    }
}
