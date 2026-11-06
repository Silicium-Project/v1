use std::time::Duration;

/// Bornes réseau. Chaque dépassement est refusé ou ignoré, jamais mis en file sans limite.
#[derive(Debug, Clone)]
pub struct NetLimits {
    /// Taille maximale d'un message d'événements (annonce, jobs).
    pub max_event_message_bytes: usize,
    /// Taille maximale d'un message de présence.
    pub max_presence_bytes: usize,
    /// Messages gossip par pair : rafale et débit soutenu par seconde.
    pub gossip_burst: u32,
    pub gossip_per_sec: u32,
    /// Requêtes de rattrapage par pair.
    pub ledger_burst: u32,
    pub ledger_per_sec: u32,
    /// Requêtes de contenu par pair (un morceau = 1).
    pub content_burst: u32,
    pub content_per_sec: u32,
    /// Pairs suivis par les limiteurs.
    pub max_tracked_peers: usize,
    /// Connexions établies au total, par pair, et en attente entrantes.
    pub max_established: u32,
    pub max_per_peer: u32,
    pub max_pending_incoming: u32,
    pub request_timeout: Duration,
    /// Événements renvoyés au plus par requête de rattrapage.
    pub max_ledger_events: usize,
}

impl Default for NetLimits {
    fn default() -> Self {
        Self {
            max_event_message_bytes: 40 * 1024,
            max_presence_bytes: 512,
            gossip_burst: 120,
            gossip_per_sec: 40,
            ledger_burst: 10,
            ledger_per_sec: 2,
            content_burst: 64,
            content_per_sec: 32,
            max_tracked_peers: 4096,
            max_established: 256,
            max_per_peer: 2,
            max_pending_incoming: 64,
            request_timeout: Duration::from_secs(30),
            max_ledger_events: 64,
        }
    }
}

/// Taille maximale d'un morceau de contenu en réponse (256 Kio + enveloppe).
pub(crate) const CHUNK_RESPONSE_MAX: u64 = 256 * 1024 + 4096;
/// Taille maximale d'un manifeste en réponse.
pub(crate) const MANIFEST_RESPONSE_MAX: u64 = 1024 * 1024;
/// Taille maximale d'une requête (toutes sont minuscules).
pub(crate) const REQUEST_MAX: u64 = 1024;
