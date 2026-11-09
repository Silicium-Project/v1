use crate::{Event, job::JobLedger};
use sil_types::{Hash32, JobId, PublicKey};
use std::collections::BTreeMap;

/// Résultat de la réception d'un événement.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Receipt {
    /// Événements nouvellement stockés (l'événement reçu et les suivants qu'il débloque),
    /// dans l'ordre d'insertion : à persister et à relayer.
    pub accepted: Vec<Event>,
    /// L'événement attend un prédécesseur manquant.
    pub pending: bool,
}

impl Receipt {
    /// Reçu d'une réception dans `job` : les événements `ids` stockés, et s'il attend encore.
    pub(crate) fn of(
        jobs: &BTreeMap<JobId, JobLedger>,
        job: &JobId,
        ids: Vec<Hash32>,
        pending: bool,
    ) -> Self {
        let accepted = jobs
            .get(job)
            .map(|ledger| {
                ids.iter()
                    .filter_map(|id| ledger.events.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            accepted,
            pending: pending && ids.is_empty(),
        }
    }
}

/// Trou à combler auprès d'un pair : événements de `author` après `after_sequence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub job: JobId,
    pub author: PublicKey,
    pub after_sequence: u64,
}
