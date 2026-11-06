//! Ce que le réseau demande au reste du nœud. Appels synchrones et rapides : ils
//! s'exécutent dans la boucle réseau (lecture mémoire ou un morceau sur disque).

use crate::Topic;
use sil_types::{ContentId, JobId, PublicKey};
use std::sync::Arc;

/// Verdict sur un message gossip, avant relais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Valide : livré à l'application et relayé.
    Accept,
    /// Invalide (mal formé, signature fausse) : non relayé, l'émetteur est pénalisé.
    Reject,
    /// Pas invalide mais inutile ou prématuré (job inconnu) : non relayé, sans pénalité.
    Ignore,
}

pub trait MessageValidator: Send + Sync {
    fn validate(&self, topic: Topic, data: &[u8]) -> Verdict;
}

pub trait LedgerProvider: Send + Sync {
    /// Événements encodés de `author` après `after`, en ordre, au plus `max`.
    fn events_after(&self, job: &JobId, author: &PublicKey, after: u64, max: usize)
    -> Vec<Vec<u8>>;

    /// Événements du job à partir de la position `cursor` du journal local, en ordre
    /// d'acceptation. Par défaut, rien : un nœud qui ne se synchronise pas ne sert pas ce mode.
    fn events_since(&self, _job: &JobId, _cursor: u64, _max: usize) -> Vec<Vec<u8>> {
        Vec::new()
    }
}

pub trait ContentProvider: Send + Sync {
    fn manifest(&self, id: &ContentId) -> Option<Vec<u8>>;
    fn chunk(&self, id: &ContentId, index: u32) -> Option<Vec<u8>>;
}

/// Accepte tout (tests, nœud sans règle).
#[derive(Debug, Clone, Copy, Default)]
pub struct AcceptAll;

impl MessageValidator for AcceptAll {
    fn validate(&self, _topic: Topic, _data: &[u8]) -> Verdict {
        Verdict::Accept
    }
}

/// Ne sert aucun événement.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLedger;

impl LedgerProvider for NoLedger {
    fn events_after(&self, _: &JobId, _: &PublicKey, _: u64, _: usize) -> Vec<Vec<u8>> {
        Vec::new()
    }
}

/// Ne sert aucun contenu.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoContent;

impl ContentProvider for NoContent {
    fn manifest(&self, _: &ContentId) -> Option<Vec<u8>> {
        None
    }

    fn chunk(&self, _: &ContentId, _: u32) -> Option<Vec<u8>> {
        None
    }
}

#[derive(Clone)]
pub struct Providers {
    pub ledger: Arc<dyn LedgerProvider>,
    pub content: Arc<dyn ContentProvider>,
    pub validator: Arc<dyn MessageValidator>,
}

impl Providers {
    /// Pour un nœud qui ne sert rien et accepte tout.
    pub fn none() -> Self {
        Self {
            ledger: Arc::new(NoLedger),
            content: Arc::new(NoContent),
            validator: Arc::new(AcceptAll),
        }
    }
}
