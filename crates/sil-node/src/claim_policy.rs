//! Politique de réclamation : plafond de tâches ouvertes, vitesse des pairs, délai des nœuds lents.
//! Décisions sur l'état du ledger et des présences ; pas d'accès réseau ni chaîne.

use crate::shared::{AttemptSeen, Shared, WorkBook, lock};
use sil_ledger::AttemptState;
use sil_types::JobId;
use std::{collections::BTreeMap, sync::Arc};

/// Fragments que ce noeud tient maintenant : reclames, ni soumis ni finals. Le plafond de
/// reputation (`max_parallel`) s'applique a ce compte, pas seulement a l'affichage.
pub(crate) fn open_claims(shared: &Arc<Shared>, me: &sil_types::PublicKey, now: u64) -> usize {
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
pub(crate) const START_PARALLEL: usize = 4;

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
pub(crate) fn live_compute_workers(shared: &Shared, now: u64) -> usize {
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
pub(crate) const PRESENCE_FRESH_MS: u64 = 60_000;
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
pub(crate) const SLOW_PERCENT: u64 = 150;
/// Délai avant qu'un nœud lent réclame un fragment resté sans preneur, quand un pair rapide existe.
pub(crate) const SLOW_GRACE_MS: u64 = 15_000;
/// Durées retenues par nœud, et minimum pour juger sa vitesse.
pub(crate) const SPEED_SAMPLES: usize = 8;
pub(crate) const SPEED_MIN_SAMPLES: usize = 2;
/// Un pair compte comme rapide seulement s'il a réclamé récemment.
pub(crate) const FAST_RECENT_MS: u64 = 120_000;
/// Nombre maximal de tentatives gardées en mémoire pour mesurer les pairs.
pub(crate) const ATTEMPTS_SEEN_MAX: usize = 4096;

/// Met à jour ce que ce nœud observe d'un fragment : tentatives vues (heure locale) et attente.
pub(crate) fn observe(
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
pub(crate) fn prune_attempts(book: &mut WorkBook) {
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
pub(crate) fn speed_of(book: &WorkBook, who: &sil_types::PublicKey) -> Option<(u64, u64)> {
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
pub(crate) fn may_claim_now(
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

/// Ce nœud peut réclamer : la chaîne répond (ancrage des claims). Sans chaîne, il ne réclame rien.
pub(crate) fn can_claim(shared: &Shared) -> bool {
    shared.chain.current_slot().is_ok()
}
