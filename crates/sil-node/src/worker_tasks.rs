//! Tâches de ce nœud comme worker : celles qu'il tient encore, et l'issue des tentatives qu'il a
//! menées. Déduit de l'état du journal ; rien n'est écrit.

use serde_json::{Value, json};
use sil_ledger::{AttemptState, JobState, TaskState, TaskStatus};
use sil_types::PublicKey;

/// Issues renvoyées au plus (les plus récentes d'abord).
pub(crate) const HISTORY: usize = 50;

/// Nom stable d'un statut de tâche, tel que l'API et l'interface le montrent.
pub(crate) fn status_name(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "open",
        TaskStatus::Claimed => "claimed",
        TaskStatus::LeaseExpired => "lease_expired",
        TaskStatus::Released => "released",
        TaskStatus::Completed => "completed",
        TaskStatus::Verified => "verified",
        TaskStatus::Disputed => "disputed",
        TaskStatus::Rejected => "rejected",
        TaskStatus::Failed => "failed",
        TaskStatus::Finalized => "finalized",
        TaskStatus::Cancelled => "cancelled",
        TaskStatus::Expired => "expired",
    }
}

/// Ce que ce worker fait dans un job : tâches qu'il tient, et issues de ses tentatives passées.
#[derive(Default)]
pub(crate) struct JobParts {
    pub(crate) running: Vec<Value>,
    /// (heure, entrée) : l'appelant trie par heure décroissante.
    pub(crate) history: Vec<(u64, Value)>,
    /// Tentatives de ce worker retenues par la finalisation.
    pub(crate) finalized: u64,
    /// Autres tentatives de ce worker, issues sans règlement.
    pub(crate) other: u64,
}

/// Les tâches de ce worker dans un job.
pub(crate) fn of_job(state: &JobState, me: &PublicKey) -> JobParts {
    let mut parts = JobParts::default();
    let job = state.job.to_hex();
    for task in &state.tasks {
        let held = holding(task, me);
        if let Some(attempt) = held {
            parts.running.push(json!({
                "job": job,
                "fragment": task.task.fragment,
                "attempt": attempt.attempt,
                "status": status_name(task.status),
                "submitted": attempt.result.is_some(),
                "started_at_ms": attempt.accepted_at_ms,
                "lease_expires_at_ms": attempt.lease_expires_at_ms,
            }));
        }
        for attempt in task.attempts.iter().filter(|a| a.claimant == *me) {
            if held.is_some_and(|h| h.attempt == attempt.attempt) {
                continue;
            }
            let (outcome, at_ms) = outcome(task, attempt);
            if outcome == "finalized" {
                parts.finalized += 1;
            } else {
                parts.other += 1;
            }
            parts.history.push((
                at_ms,
                json!({
                    "job": job,
                    "fragment": task.task.fragment,
                    "attempt": attempt.attempt,
                    "outcome": outcome,
                    "at_ms": at_ms,
                    "approvals": attempt.verifiers.iter().filter(|v| v.verdict == Some(true)).count(),
                }),
            ));
        }
    }
    parts
}

/// La tâche est tenue par ce worker si sa dernière tentative est la sienne, non libérée, et que
/// la tâche est encore en cours (ni finalisée, ni échouée).
fn holding<'a>(task: &'a TaskState, me: &PublicKey) -> Option<&'a AttemptState> {
    let last = task.attempts.last()?;
    let live = matches!(
        task.status,
        TaskStatus::Claimed | TaskStatus::Completed | TaskStatus::Verified | TaskStatus::Disputed
    );
    (last.claimant == *me && !last.released && task.finalized.is_none() && live).then_some(last)
}

/// Issue d'une tentative menée par ce worker, et l'heure qui la date.
fn outcome(task: &TaskState, attempt: &AttemptState) -> (&'static str, u64) {
    match &task.finalized {
        Some(done) if done.attempt == attempt.attempt => (
            "finalized",
            attempt
                .result
                .as_ref()
                .map_or(attempt.accepted_at_ms, |result| result.submitted_at_ms),
        ),
        Some(_) => ("superseded", attempt.accepted_at_ms),
        None if attempt.released => ("released", attempt.accepted_at_ms),
        None if attempt.verifiers.iter().any(|v| v.verdict == Some(false)) => {
            ("rejected", attempt.accepted_at_ms)
        }
        None => ("expired", attempt.accepted_at_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_ledger::{Finalized, JobStatus, ResultState, VerifierState};
    use sil_types::{Hash32, JobId, TaskRef};

    fn key(n: u8) -> PublicKey {
        PublicKey::from_bytes([n; 32])
    }

    fn attempt(n: u32, claimant: PublicKey) -> AttemptState {
        AttemptState {
            attempt: n,
            claimant,
            claim_event: Hash32::from_bytes([n as u8; 32]),
            accepted_at_ms: 1_000 * u64::from(n),
            renewals: 0,
            lease_expires_at_ms: 9_000,
            released: false,
            result: None,
            verifiers: Vec::new(),
        }
    }

    fn submitted(mut attempt: AttemptState, at_ms: u64) -> AttemptState {
        attempt.result = Some(ResultState {
            event: Hash32::from_bytes([9; 32]),
            result: Hash32::from_bytes([8; 32]),
            size: 4,
            milestones_root: Hash32::from_bytes([7; 32]),
            submitted_at_ms: at_ms,
        });
        attempt
    }

    fn task(
        status: TaskStatus,
        attempts: Vec<AttemptState>,
        finalized: Option<Finalized>,
    ) -> TaskState {
        TaskState {
            task: TaskRef::new(JobId::from_bytes([7; 32]), 0),
            status,
            attempts,
            next_attempt: None,
            finalized,
        }
    }

    fn job(tasks: Vec<TaskState>) -> JobState {
        JobState {
            job: JobId::from_bytes([7; 32]),
            status: JobStatus::Active,
            tasks,
        }
    }

    #[test]
    fn a_running_attempt_is_listed_until_it_is_submitted() {
        let me = key(1);
        let parts = of_job(
            &job(vec![task(TaskStatus::Claimed, vec![attempt(1, me)], None)]),
            &me,
        );
        assert_eq!(parts.running.len(), 1);
        assert_eq!(parts.running[0]["status"], "claimed");
        assert_eq!(parts.running[0]["submitted"], false);
        assert!(parts.history.is_empty());
    }

    #[test]
    fn a_submitted_attempt_waits_for_verification_and_stays_in_progress() {
        let me = key(1);
        let parts = of_job(
            &job(vec![task(
                TaskStatus::Completed,
                vec![submitted(attempt(1, me), 5_000)],
                None,
            )]),
            &me,
        );
        assert_eq!(parts.running.len(), 1);
        assert_eq!(parts.running[0]["status"], "completed");
        assert_eq!(parts.running[0]["submitted"], true);
    }

    #[test]
    fn other_workers_tasks_are_not_mine() {
        let me = key(1);
        let parts = of_job(
            &job(vec![task(
                TaskStatus::Claimed,
                vec![attempt(1, key(2))],
                None,
            )]),
            &me,
        );
        assert!(parts.running.is_empty());
        assert!(parts.history.is_empty());
    }

    #[test]
    fn my_finalized_attempt_is_paid_history() {
        let me = key(1);
        let done = Finalized {
            attempt: 1,
            result: Hash32::from_bytes([8; 32]),
            claimant: me,
            approvers: vec![key(3)],
        };
        let parts = of_job(
            &job(vec![task(
                TaskStatus::Finalized,
                vec![submitted(attempt(1, me), 5_000)],
                Some(done),
            )]),
            &me,
        );
        assert!(parts.running.is_empty());
        assert_eq!((parts.finalized, parts.other), (1, 0));
        assert_eq!(parts.history[0].0, 5_000);
        assert_eq!(parts.history[0].1["outcome"], "finalized");
    }

    #[test]
    fn an_attempt_superseded_or_rejected_or_released_is_not_paid() {
        let me = key(1);
        let done = Finalized {
            attempt: 2,
            result: Hash32::from_bytes([8; 32]),
            claimant: key(2),
            approvers: Vec::new(),
        };
        let mut rejected = attempt(1, me);
        rejected.verifiers.push(VerifierState {
            verifier: key(4),
            offer_event: Hash32::from_bytes([1; 32]),
            accepted_at_ms: 1,
            verdict: Some(false),
        });
        let mut released = attempt(3, me);
        released.released = true;
        let parts = of_job(
            &job(vec![
                task(
                    TaskStatus::Finalized,
                    vec![attempt(1, me), attempt(2, key(2))],
                    Some(done),
                ),
                task(TaskStatus::Rejected, vec![rejected], None),
                task(TaskStatus::Released, vec![released], None),
            ]),
            &me,
        );
        let outcomes: Vec<&str> = parts
            .history
            .iter()
            .filter_map(|(_, entry)| entry["outcome"].as_str())
            .collect();
        assert_eq!(outcomes.len(), 3);
        assert!(outcomes.contains(&"superseded"));
        assert!(outcomes.contains(&"rejected"));
        assert!(outcomes.contains(&"released"));
        assert_eq!((parts.finalized, parts.other), (0, 3));
    }
}
