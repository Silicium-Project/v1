//! Alimente la réputation locale avec les issues observées dans le journal.

use crate::shared::{Shared, lock};
use sil_ledger::TaskStatus;
use sil_reputation::{Outcome, verdict_outcome};
use sil_settlement::terms_of;
use sil_types::JobId;

/// Compte les tentatives terminées d'un job : une tentative finalisée est un succès, une
/// tentative remplacée par une suivante (bail expiré, abandon, rejet) ou la dernière tentative
/// échouée est un échec. Chaque tentative n'est comptée qu'une fois.
///
/// Une réussite pèse la valeur de son fragment : unités × tarif par unité.
///
/// Sans effet si ce nœud ne croit pas les journaux de ce requester.
pub(crate) fn observe_outcomes(shared: &Shared, job: &JobId) {
    let now = shared.now_ms();
    let Some(spec) = lock(&shared.ledger).spec(job).map(|s| s.spec.clone()) else {
        return;
    };
    if !lock(&shared.reputation).trusts(&spec.requester)
    {
        return;
    }
    let terms = terms_of(&spec);
    for fragment in 0..spec.fragments {
        // Un job sans règle d'unités ne peut pas être réglé : rien à compter.
        let Ok(units) = terms.fragment_units(fragment) else {
            continue;
        };
        let Some(state) = lock(&shared.ledger).task_state(job, fragment, now) else {
            continue;
        };
        let count = state.attempts.len();
        let last_failed = matches!(
            state.status,
            TaskStatus::LeaseExpired
                | TaskStatus::Released
                | TaskStatus::Rejected
                | TaskStatus::Failed
        );
        for (index, attempt) in state.attempts.iter().enumerate() {
            let won = state
                .finalized
                .as_ref()
                .is_some_and(|f| f.attempt == attempt.attempt);
            let outcome = if won {
                Outcome::Completed {
                    reward: units.saturating_mul(spec.reward_per_unit),
                }
            } else if index + 1 < count || (index + 1 == count && last_failed) {
                Outcome::Failed
            } else {
                continue;
            };
            let key = (*job, fragment, attempt.attempt);
            if lock(&shared.work).observed.insert(key) {
                lock(&shared.reputation).observe(
                    &spec.requester,
                    &attempt.claimant,
                    &[],
                    outcome,
                    now,
                );
            }
            // Verdicts : un vérificateur a raison quand son verdict suit l'issue de la tentative.
            for verifier in &attempt.verifiers {
                let Some(approved) = verifier.verdict else {
                    continue;
                };
                let key = (*job, fragment, attempt.attempt, verifier.verifier);
                if lock(&shared.work).observed_verdicts.insert(key) {
                    lock(&shared.verify_reputation).observe(
                        &spec.requester,
                        &verifier.verifier,
                        &[],
                        verdict_outcome(approved, won),
                        now,
                    );
                }
            }
        }
    }
}
