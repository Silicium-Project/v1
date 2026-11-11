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

/// Image d'un job de rendu : pixels RGB24 ligne par ligne, du haut vers le bas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobImage {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// Résultat brut d'un fragment (octets tels que le workload les a produits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentOutput {
    pub fragment: u32,
    pub bytes: Vec<u8>,
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

    /// Image assemblée d'un job de rendu : tous les fragments doivent être finalisés. Refusé
    /// (`BadRequest`) si le workload ne produit pas d'image, `Unavailable` tant que ce n'est pas
    /// final ou si un contenu manque.
    fn job_image(&self, job: &JobId) -> Result<JobImage, ApiError>;

    /// Résultat brut de chaque fragment d'un job final, dans l'ordre des fragments. Refusé
    /// (`Unavailable`) tant qu'un fragment n'est pas final, et (`BadRequest`) si un résultat est
    /// trop grand pour être donné par l'API (un résultat d'image passe par `job_image`).
    fn job_outputs(&self, job: &JobId) -> Result<Vec<FragmentOutput>, ApiError>;

    /// Contenu d'entrée d'un job, tel que le nœud le stocke (octets bruts). Réservé au
    /// propriétaire côté API. `NotFound` si le job est inconnu.
    fn job_input(&self, job: &JobId) -> Result<Vec<u8>, ApiError>;

    /// Résultats de tous les fragments d'un job final, concaténés dans l'ordre (octets bruts).
    /// Pour les workloads de lots : chaque fragment est un cadre (`sil-batch`), la concaténation
    /// reste un cadre valide. Refusé si le résultat dépasse la limite de l'archive.
    fn job_archive(&self, job: &JobId) -> Result<Vec<u8>, ApiError>;

    /// Soumet un règlement signé (octets CBOR d'une `Authorization`) avec les frais de ce nœud.
    /// Retourne `{ "tx": ... }`. Refusé si le fragment est déjà réglé ou si ce nœud ne paie pas
    /// les frais (relais : voir `Chain::pays_fees`).
    fn submit_settlement(&self, bytes: Vec<u8>) -> Result<Value, ApiError>;

    fn peers(&self) -> Result<Value, ApiError>;

    /// État des règlements (`Pending`, `Submitted`, `Unknown`, `Confirmed`…).
    fn settlements(&self) -> Result<Value, ApiError>;

    /// Activité de ce nœud comme worker : événements, réputation, règlements vus.
    fn worker(&self) -> Result<Value, ApiError>;
}
