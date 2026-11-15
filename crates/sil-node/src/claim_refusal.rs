//! Journal des refus de claim : pourquoi ce nœud ne réclame pas un fragment ouvert.
//! Niveau `info`, au plus une ligne par (job, fragment, raison) toutes les 60 secondes.

use crate::shared::{Shared, lock};
use sil_types::JobId;

/// Fragment désigné quand le refus porte sur le job entier (par exemple l'escrow).
pub(crate) const WHOLE_JOB: u32 = u32::MAX;
/// Délai minimal entre deux lignes identiques.
const REFUSAL_LOG_MS: u64 = 60_000;

/// Journalise un refus de claim, sauf s'il a déjà été journalisé récemment.
pub(crate) fn refuse(
    shared: &Shared,
    job: &JobId,
    fragment: u32,
    reason: &'static str,
    detail: &str,
) {
    let now = shared.now_ms();
    {
        let mut book = lock(&shared.work);
        let key = (*job, fragment, reason);
        if book
            .refusals
            .get(&key)
            .is_some_and(|last| now.saturating_sub(*last) < REFUSAL_LOG_MS)
        {
            return;
        }
        book.refusals.insert(key, now);
    }
    tracing::info!(%job, fragment, reason, detail, "claim refusé");
}
