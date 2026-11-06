use crate::ApiError;
use serde_json::Value;
use sil_types::{ContentId, JobId};

/// Résultat de l'envoi d'un événement signé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventReceipt {
    /// Événements nouvellement stockés (l'événement et ceux qu'il débloque).
    pub accepted: usize,
    /// L'événement attend un prédécesseur manquant.
    pub pending: bool,
}

/// Ce que l'API demande au nœud. Méthodes bloquantes : l'API les appelle depuis un thread
/// dédié, jamais dans la boucle asynchrone.
pub trait Backend: Send + Sync + 'static {
    /// Stocke un contenu (envoi par octets) et l'annonce aux pairs.
    fn put_content(&self, bytes: Vec<u8>) -> Result<ContentId, ApiError>;

    /// Reçoit un événement signé (octets canoniques) : vérifie, stocke, relaie.
    fn submit_event(&self, bytes: Vec<u8>) -> Result<EventReceipt, ApiError>;

    /// État dérivé d'un job, en JSON (`None` s'il est inconnu).
    fn job(&self, job: &JobId) -> Result<Option<Value>, ApiError>;

    fn peers(&self) -> Result<Value, ApiError>;

    /// État des règlements (`Pending`, `Submitted`, `Unknown`, `Confirmed`…).
    fn settlements(&self) -> Result<Value, ApiError>;

    /// Activité de ce nœud comme worker : événements, réputation, règlements vus.
    fn worker(&self) -> Result<Value, ApiError>;
}
