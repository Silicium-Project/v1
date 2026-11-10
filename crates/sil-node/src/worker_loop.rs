//! Worker : réclame les fragments ouverts (si l'escrow est sain), lance ceux qu'on lui a attribués.

use crate::{
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

pub(crate) async fn run(shared: Arc<Shared>, tick: Duration, mut stop: watch::Receiver<bool>) {
    let mut timer = interval(tick);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = timer.tick() => {
                for job in shared.job_ids() {
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
    for fragment in 0..spec.fragments {
        let Some(state) = lock(&shared.ledger).task_state(job, fragment, now) else {
            continue;
        };
        if let Some(attempt) = state.next_attempt
            && !state.attempts.iter().any(|a| a.claimant == me)
            && open_claims(shared, &me, now) < max_parallel(shared, &me, now)
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
            if ok {
                let _ = shared.author_event(job, EventBody::Claim { fragment, attempt });
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
    let ok = escrow.is_some_and(|escrow| check_escrow(spec, &escrow, now, needed_ms).is_ok());
    lock(&shared.work)
        .escrow_ok
        .insert(*job, (now + ESCROW_TTL_MS, ok));
    ok
}

/// Fragments que ce noeud tient maintenant : reclames, ni soumis ni finals. Le plafond de
/// reputation (`max_parallel`) s'applique a ce compte, pas seulement a l'affichage.
fn open_claims(shared: &Arc<Shared>, me: &sil_types::PublicKey, now: u64) -> usize {
    let mut open = 0;
    for job in shared.job_ids() {
        let Some(fragments) = lock(&shared.ledger).spec(&job).map(|s| s.spec.fragments) else {
            continue;
        };
        for fragment in 0..fragments {
            let Some(state) = lock(&shared.ledger).task_state(&job, fragment, now) else {
                continue;
            };
            if state.finalized.is_none()
                && let Some(last) = state.attempts.last()
                && last.claimant == *me
                && last.result.is_none()
                && !last.released
            {
                open += 1;
            }
        }
    }
    open
}

/// Taches simultanees de depart : un noeud neuf en fait autant. La reputation ne fait que relever
/// ce plancher, sans jamais l'interdire (une reputation nulle, elle, interdit toute tache).
const START_PARALLEL: usize = 4;

/// Taches simultanees de ce noeud : la reputation, bornee par les coeurs (un coeur reste libre pour
/// le systeme et l'interface). Le rendu se fait sur plusieurs coeurs (sil-raytracer, render.rs).
pub(crate) fn max_parallel(shared: &Shared, me: &sil_types::PublicKey, now: u64) -> usize {
    let reputation = lock(&shared.reputation).max_parallel(me, now) as usize;
    allowed_parallel(reputation, cpu_limit())
}

/// Plafond pur : 0 si la reputation l'interdit, sinon au moins `START_PARALLEL`, borne aux coeurs.
pub(crate) fn allowed_parallel(reputation: usize, cpu: usize) -> usize {
    if reputation == 0 {
        return 0;
    }
    reputation.max(START_PARALLEL).min(cpu)
}

pub(crate) fn cpu_limit() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_node_runs_its_start_count_of_tasks() {
        assert_eq!(allowed_parallel(1, 15), START_PARALLEL);
    }

    #[test]
    fn reputation_raises_the_cap_up_to_the_cores() {
        assert_eq!(allowed_parallel(6, 15), 6);
        assert_eq!(allowed_parallel(8, 3), 3);
    }

    #[test]
    fn a_banned_worker_gets_no_task() {
        assert_eq!(allowed_parallel(0, 15), 0);
    }
}
