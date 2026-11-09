//! Observations tirées du journal : une tentative gagnée ou perdue, un verdict juste ou faux.
//! Même règle que `reputation_feed`, mais relue entièrement dans le journal, à l'heure du job.

use super::clock::job_clock;
use sil_ledger::{Ledger, TaskStatus};
use sil_reputation::{Observation, Outcome, verdict_outcome};
use sil_settlement::terms_of;

/// Toutes les observations du journal, et l'heure de jugement : la plus grande horloge de job.
pub(crate) fn collect(ledger: &Ledger) -> (Vec<Observation>, u64) {
    let mut out = Vec::new();
    let mut as_of_ms = 0;
    for job in ledger.job_ids() {
        let Some(signed) = ledger.spec(&job) else {
            continue;
        };
        let spec = &signed.spec;
        let clock = job_clock(ledger, &job);
        as_of_ms = as_of_ms.max(clock);
        let terms = terms_of(spec);
        for fragment in 0..spec.fragments {
            // Un fragment sans règle d'unités ne peut pas être réglé : rien à compter.
            let Ok(units) = terms.fragment_units(fragment) else {
                continue;
            };
            let Some(state) = ledger.task_state(&job, fragment, clock) else {
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
                    // Tentative encore en cours : rien à dire tant que son bail court.
                    continue;
                };
                let at_ms = attempt
                    .result
                    .as_ref()
                    .map_or(attempt.accepted_at_ms, |result| result.submitted_at_ms);
                out.push(Observation {
                    scope: spec.requester,
                    who: attempt.claimant,
                    insiders: Vec::new(),
                    outcome,
                    at_ms,
                });
                for verifier in &attempt.verifiers {
                    let Some(approved) = verifier.verdict else {
                        continue;
                    };
                    out.push(Observation {
                        scope: spec.requester,
                        who: verifier.verifier,
                        insiders: Vec::new(),
                        outcome: verdict_outcome(approved, won),
                        at_ms: verifier.accepted_at_ms,
                    });
                }
            }
        }
    }
    (out, as_of_ms)
}
