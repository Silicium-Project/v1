use crate::Event;
use sil_types::{JobId, PublicKey};

/// Résultat de la réception d'un événement.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Receipt {
    /// Événements nouvellement stockés (l'événement reçu et les suivants qu'il débloque),
    /// dans l'ordre d'insertion : à persister et à relayer.
    pub accepted: Vec<Event>,
    /// L'événement attend un prédécesseur manquant.
    pub pending: bool,
}

/// Trou à combler auprès d'un pair : événements de `author` après `after_sequence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub job: JobId,
    pub author: PublicKey,
    pub after_sequence: u64,
}
