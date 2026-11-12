//! Worker : réclame les fragments ouverts (si l'escrow est sain), lance ceux qu'on lui a attribués.

use crate::{
    shared::{AttemptSeen, Shared, WorkBook, lock},
    work,
};
use sil_ledger::{AttemptState, EventBody, TaskStatus};
use sil_settlement::check_escrow;
use sil_types::JobId;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
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
            && open_claims(shared, &me, now) < cap
            && may_claim_now(shared, &me, job, fragment, now)
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
                && lease_held(last.lease_expires_at_ms, now)
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

/// Un claim compte comme tache en cours tant que son bail court. Un bail expire (noeud arrete en pleine
/// tache) ne bloque plus ce noeud : le reseau peut reprendre le fragment.
pub(crate) fn lease_held(lease_expires_at_ms: u64, now: u64) -> bool {
    lease_expires_at_ms > now
}

/// Nombre de noeuds qui calculent (ce noeud compris) et ont donne signe de vie recemment.
fn live_compute_workers(shared: &Shared, now: u64) -> usize {
    let work = lock(&shared.work);
    compute_peers_alive(&work.presence, now) + 1
}

/// Pairs qui calculent et dont la dernière présence date de moins de `PRESENCE_FRESH_MS`.
pub(crate) fn compute_peers_alive(
    presence: &BTreeMap<sil_net::PeerId, (sil_net::Presence, u64)>,
    now: u64,
) -> usize {
    presence
        .values()
        .filter(|(presence, seen)| {
            presence.roles & sil_net::ROLE_COMPUTE != 0
                && now.saturating_sub(*seen) < PRESENCE_FRESH_MS
        })
        .count()
}

/// Un noeud dont la presence date de plus de cette duree n'est plus compte comme actif.
/// Au moins 4 fois la période de présence : trois présences perdues d'affilée restent tolérées.
const PRESENCE_FRESH_MS: u64 = 60_000;
const _: () = assert!(PRESENCE_FRESH_MS >= 4 * crate::presence_loop::MAX_SILENCE_MS);

/// Une seule tache a la fois tant qu'il reste des fragments sans preneur et qu'un autre noeud calcule ;
/// sinon le plafond normal. Un noeud seul garde son parallelisme (personne d'autre ne prendra les taches).
pub(crate) fn claim_cap(waiting: usize, live_workers: usize, max: usize) -> usize {
    if waiting > 0 && live_workers > 1 {
        max.min(1)
    } else {
        max
    }
}

/// Un nœud est lent si sa médiane dépasse celle d'un pair rapide de plus de 50 % (150 %).
const SLOW_PERCENT: u64 = 150;
/// Délai avant qu'un nœud lent réclame un fragment resté sans preneur, quand un pair rapide existe.
const SLOW_GRACE_MS: u64 = 15_000;
/// Durées retenues par nœud, et minimum pour juger sa vitesse.
const SPEED_SAMPLES: usize = 8;
const SPEED_MIN_SAMPLES: usize = 2;
/// Un pair compte comme rapide seulement s'il a réclamé récemment.
const FAST_RECENT_MS: u64 = 120_000;
/// Nombre maximal de tentatives gardées en mémoire pour mesurer les pairs.
const ATTEMPTS_SEEN_MAX: usize = 4096;

/// Met à jour ce que ce nœud observe d'un fragment : tentatives vues (heure locale) et attente.
fn observe(
    shared: &Shared,
    job: &JobId,
    fragment: u32,
    attempts: &[AttemptState],
    waiting: bool,
    now: u64,
) {
    let mut book = lock(&shared.work);
    if waiting {
        book.waiting_since.entry((*job, fragment)).or_insert(now);
    } else {
        book.waiting_since.remove(&(*job, fragment));
    }
    for attempt in attempts {
        let key = (*job, fragment, attempt.attempt);
        let has_result = attempt.result.is_some();
        let seen = book.attempt_seen.entry(key).or_insert_with(|| AttemptSeen {
            claimant: attempt.claimant,
            claim_ms: (!has_result).then_some(now),
            result_ms: None,
        });
        if has_result && seen.result_ms.is_none() && seen.claim_ms.is_some() {
            seen.result_ms = Some(now);
        }
    }
    prune_attempts(&mut book);
}

/// Garde au plus `ATTEMPTS_SEEN_MAX` tentatives, les plus anciennes d'abord.
fn prune_attempts(book: &mut WorkBook) {
    if book.attempt_seen.len() <= ATTEMPTS_SEEN_MAX {
        return;
    }
    let mut ages: Vec<(u64, (JobId, u32, u32))> = book
        .attempt_seen
        .iter()
        .map(|(key, seen)| (seen.claim_ms.unwrap_or(0), *key))
        .collect();
    ages.sort_unstable();
    let excess = book.attempt_seen.len() - ATTEMPTS_SEEN_MAX;
    for (_, key) in ages.into_iter().take(excess) {
        book.attempt_seen.remove(&key);
    }
}

/// Vitesse d'un nœud à partir de ses durées (claim -> résultat, en ms) : médiane des dernières
/// tentatives et heure du dernier claim. `None` tant qu'il n'y a pas assez de mesures.
pub(crate) fn speed_from(samples: &[(u64, u64)]) -> Option<(u64, u64)> {
    if samples.len() < SPEED_MIN_SAMPLES {
        return None;
    }
    let mut by_claim = samples.to_vec();
    by_claim.sort_unstable();
    let recent = &by_claim[by_claim.len().saturating_sub(SPEED_SAMPLES)..];
    let last_claim = recent.last()?.0;
    let mut durations: Vec<u64> = recent
        .iter()
        .map(|(claim, result)| result.saturating_sub(*claim))
        .collect();
    durations.sort_unstable();
    Some((durations[durations.len() / 2], last_claim))
}

/// Vitesse observée d'un nœud dans le carnet local.
fn speed_of(book: &WorkBook, who: &sil_types::PublicKey) -> Option<(u64, u64)> {
    let samples: Vec<(u64, u64)> = book
        .attempt_seen
        .values()
        .filter(|seen| seen.claimant == *who)
        .filter_map(|seen| Some((seen.claim_ms?, seen.result_ms?)))
        .collect();
    speed_from(&samples)
}

/// Vrai si `mine` dépasse `fastest` de plus de la marge (`SLOW_PERCENT`).
pub(crate) fn slower(mine: u64, fastest: u64) -> bool {
    mine.saturating_mul(100) > fastest.saturating_mul(SLOW_PERCENT)
}

/// Un nœud plus lent qu'un pair rapide récent attend `SLOW_GRACE_MS` après l'apparition du fragment,
/// pour laisser le pair rapide le prendre. Le délai est borné : un fragment n'est jamais bloqué.
fn may_claim_now(
    shared: &Shared,
    me: &sil_types::PublicKey,
    job: &JobId,
    fragment: u32,
    now: u64,
) -> bool {
    let book = lock(&shared.work);
    let Some((mine, _)) = speed_of(&book, me) else {
        return true;
    };
    let mut others: Vec<sil_types::PublicKey> = Vec::new();
    for seen in book.attempt_seen.values() {
        if seen.claimant != *me && !others.contains(&seen.claimant) {
            others.push(seen.claimant);
        }
    }
    let fastest = others
        .iter()
        .filter_map(|who| speed_of(&book, who))
        .filter(|(_, last_claim)| now.saturating_sub(*last_claim) < FAST_RECENT_MS)
        .map(|(median, _)| median)
        .min();
    let Some(fastest) = fastest else {
        return true;
    };
    if !slower(mine, fastest) {
        return true;
    }
    let since = book
        .waiting_since
        .get(&(*job, fragment))
        .copied()
        .unwrap_or(now);
    now.saturating_sub(since) >= SLOW_GRACE_MS
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

    #[test]
    fn with_work_waiting_and_other_workers_each_node_takes_one_task() {
        assert_eq!(claim_cap(5, 2, 3), 1);
    }

    #[test]
    fn a_lone_worker_keeps_its_parallelism() {
        assert_eq!(claim_cap(5, 1, 3), 3);
    }

    #[test]
    fn nothing_waiting_means_the_normal_cap() {
        assert_eq!(claim_cap(0, 2, 3), 3);
    }

    #[test]
    fn an_expired_lease_no_longer_counts_as_a_running_task() {
        assert!(lease_held(2_000, 1_000));
        assert!(!lease_held(1_000, 1_000));
        assert!(!lease_held(500, 1_000));
    }

    #[test]
    fn a_banned_worker_stays_at_zero() {
        assert_eq!(claim_cap(5, 2, 0), 0);
    }

    #[test]
    fn a_worker_is_still_counted_after_three_lost_presence_messages() {
        // Présence toutes les 15 s : trois messages perdus, la dernière reçue il y a 50 s.
        let peer = sil_net::PeerId::random();
        let presence = sil_net::Presence {
            version: 1,
            roles: sil_net::ROLE_COMPUTE,
            load: 0,
            workloads: Vec::new(),
        };
        let mut book = BTreeMap::new();
        book.insert(peer, (presence, 1_000_000));
        assert_eq!(compute_peers_alive(&book, 1_050_000), 1);
        // Au-delà de la fenêtre, le pair est bien compté comme parti.
        assert_eq!(compute_peers_alive(&book, 1_000_000 + PRESENCE_FRESH_MS), 0);
    }

    #[test]
    fn slower_needs_a_fifty_percent_margin() {
        assert!(!slower(100, 100));
        assert!(!slower(150, 100));
        assert!(slower(151, 100));
    }

    #[test]
    fn speed_needs_a_few_samples() {
        assert_eq!(speed_from(&[(0, 10)]), None);
    }

    #[test]
    fn speed_is_the_median_of_the_recent_durations() {
        // Durées 10, 30, 60 ms : médiane 30 ; dernier claim à 200 ms.
        assert_eq!(
            speed_from(&[(0, 10), (100, 130), (200, 260)]),
            Some((30, 200))
        );
    }

    #[test]
    fn old_slow_samples_stop_counting() {
        let mut samples: Vec<(u64, u64)> =
            (0..4).map(|i| (i * 10_000, i * 10_000 + 90_000)).collect();
        samples.extend((4..12).map(|i| (i * 10_000, i * 10_000 + 1_000)));
        let (median, _) = speed_from(&samples).expect("assez de mesures");
        assert_eq!(median, 1_000);
    }
}
