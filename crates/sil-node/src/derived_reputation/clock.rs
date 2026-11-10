//! Horloge d'un job, lue dans son journal (ADR 0005, phase 1b ; ADR 0007).
//!
//! Il n'y a plus de comité pour fixer l'heure de jugement. L'horloge est la date la plus récente
//! parmi les événements du job. Elle sert à l'affichage de la réputation dérivée, qui ne pèse sur
//! aucune décision (ADR 0005).

use sil_ledger::Ledger;
use sil_types::JobId;

/// Horloge du job : la plus grande date de dernier événement connue, par auteur. 0 si le job est
/// inconnu.
pub(crate) fn job_clock(ledger: &Ledger, job: &JobId) -> u64 {
    ledger
        .heads(job)
        .keys()
        .filter_map(|author| ledger.head_time(job, author))
        .max()
        .unwrap_or(0)
}
