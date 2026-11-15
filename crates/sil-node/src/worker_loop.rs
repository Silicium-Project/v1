//! Worker : réclame les fragments ouverts (si l'escrow est sain), lance ceux qu'on lui a attribués.

use crate::{
    claim_policy::{
        claim_cap, live_compute_workers, max_parallel, may_claim_now, observe, open_claims,
    },
    claim_refusal::{WHOLE_JOB, refuse},
    shared::{Shared, lock},
    work,
};
use sil_ledger::{EventBody, TaskStatus};
use sil_settlement::check_escrow;
use sil_types::JobId;
use std::{sync::Arc, time::Duration};
use tokio::{sync::watch, time::interval};

/// Durée pendant laquelle on réutilise le verdict de la garde d'escrow d'un job.
const ESCROW_TTL_MS: u64 = 5_000;

/// Jobs dans l'ordre de leur échéance, la plus proche d'abord : un job long, soumis tôt, passe avant
/// les petits jobs soumis après. Sans cet ordre, un flux de petits jobs peut affamer un long.
fn jobs_by_deadline(shared: &Shared) -> Vec<JobId> {
    let ledger = lock(&shared.ledger);
    let mut jobs: Vec<(u64, JobId)> = ledger
        .job_ids()
        .into_iter()
        .filter_map(|job| {
            ledger
                .spec(&job)
                .map(|signed| (signed.spec.expires_at_ms, job))
        })
        .collect();
    jobs.sort_unstable();
    jobs.into_iter().map(|(_, job)| job).collect()
}

pub(crate) async fn run(shared: Arc<Shared>, tick: Duration, mut stop: watch::Receiver<bool>) {
    let mut timer = interval(tick);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = timer.tick() => {
                for job in jobs_by_deadline(&shared) {
                    scan_job(&shared, &job).await;
                }
            }
        }
    }
}

async fn scan_job(shared: &Arc<Shared>, job: &JobId) {
    let now = shared.now_ms();
    let Some(spec) = lock(&shared.ledger).spec(job).map(|s| s.spec.clone()) else {
        return;
    };
    let Some(workload) = shared.registry.get(&spec.workload) else {
        return;
    };
    if now > spec.expires_at_ms {
        return;
    }
    let me = shared.identity.public();
    let mut escrow_checked: Option<bool> = None;
    // Fragments encore sans preneur : tant qu'il en reste et que d'autres noeuds calculent, on ne
    // prend qu'une tache a la fois (la repartition se fait par noeud, pas par plus grosse capacite).
    let waiting = (0..spec.fragments)
        .filter(|&fragment| {
            lock(&shared.ledger)
                .task_state(job, fragment, now)
                .is_some_and(|state| state.next_attempt.is_some())
        })
        .count();
    let cap = claim_cap(
        waiting,
        live_compute_workers(shared, now),
        max_parallel(shared, &me, now),
    );
    for fragment in 0..spec.fragments {
        let Some(state) = lock(&shared.ledger).task_state(job, fragment, now) else {
            continue;
        };
        observe(
            shared,
            job,
            fragment,
            &state.attempts,
            state.next_attempt.is_some(),
            now,
        );
        if let Some(attempt) = state.next_attempt
            && !state.attempts.iter().any(|a| a.claimant == me)
            && claim_allowed(shared, job, fragment, cap, waiting, now)
            && lock(&shared.work).claimed.insert((*job, fragment, attempt))
        {
            let ok = match escrow_checked {
                Some(ok) => ok,
                None => {
                    let ok = escrow_is_sound(shared, job, &spec, workload.limits().wall_ms).await;
                    escrow_checked = Some(ok);
                    ok
                }
            };
            // Ancrage (ADR 0009) : le slot courant lu sur la chaîne. Sans lecture, pas de claim.
            let anchor = match shared.chain.current_slot() {
                Ok(slot) if ok => Some(slot),
                Ok(_) => None,
                Err(error) => {
                    refuse(shared, job, fragment, "ancrage", &error.to_string());
                    None
                }
            };
            if let Some(anchor_slot) = anchor {
                let _ = shared.author_event(
                    job,
                    EventBody::Claim {
                        fragment,
                        attempt,
                        anchor_slot,
                    },
                );
            } else {
                // Pas de claim : on pourra réessayer au prochain tour.
                lock(&shared.work)
                    .claimed
                    .remove(&(*job, fragment, attempt));
            }
        }
        if let Some(last) = state.attempts.last()
            && last.claimant == me
            && last.result.is_none()
            && state.status == TaskStatus::Claimed
            && lock(&shared.work)
                .running
                .insert((*job, fragment, last.attempt))
        {
            work::spawn_execution(Arc::clone(shared), *job, fragment, last.attempt);
        }
    }
}

/// Vérifie l'escrow (avec un cache court) : on ne travaille que si on sera payé.
async fn escrow_is_sound(
    shared: &Arc<Shared>,
    job: &JobId,
    spec: &sil_ledger::JobSpec,
    needed_ms: u64,
) -> bool {
    // Mode de test : pas de paiement, donc pas d'escrow à vérifier (refusé avec une chaîne).
    if shared.free_work {
        return true;
    }
    let now = shared.now_ms();
    if let Some((until, ok)) = lock(&shared.work).escrow_ok.get(job).copied()
        && now < until
    {
        return ok;
    }
    let chain = Arc::clone(&shared.chain);
    let job_id = *job;
    let escrow = tokio::task::spawn_blocking(move || chain.escrow(&job_id))
        .await
        .ok()
        .and_then(Result::ok)
        .flatten();
    let verdict = match escrow {
        None => Err("pas d'escrow lu sur la chaîne (absent ou RPC indisponible)".to_string()),
        Some(escrow) => check_escrow(spec, &escrow, now, needed_ms).map_err(|e| e.to_string()),
    };
    if let Err(reason) = &verdict {
        refuse(shared, job, WHOLE_JOB, "escrow", reason);
    }
    let ok = verdict.is_ok();
    lock(&shared.work)
        .escrow_ok
        .insert(*job, (now + ESCROW_TTL_MS, ok));
    ok
}

/// Vrai si la politique laisse ce nœud réclamer ce fragment. Sinon, journalise le motif
/// (plafond de tâches ouvertes, ou nœud plus lent qu'un pair rapide pendant son délai de grâce).
fn claim_allowed(
    shared: &Arc<Shared>,
    job: &JobId,
    fragment: u32,
    cap: usize,
    waiting: usize,
    now: u64,
) -> bool {
    let me = shared.identity.public();
    let open = open_claims(shared, &me, now);
    if open >= cap {
        let detail = format!(
            "{open} tâche(s) ouverte(s), plafond {cap}, {waiting} fragment(s) sans preneur"
        );
        refuse(shared, job, fragment, "plafond", &detail);
        return false;
    }
    if !may_claim_now(shared, &me, job, fragment, now) {
        refuse(
            shared,
            job,
            fragment,
            "nœud lent",
            "un pair rapide a réclamé récemment : délai de grâce",
        );
        return false;
    }
    true
}
