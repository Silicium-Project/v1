//! Finaliseur (ADR 0005 phase 4, ADR 0007) : tout nœud publie le `Finalize` d'un résultat approuvé,
//! et met en file le règlement signé par les approbateurs. Il n'y a ni comité ni vote.

use crate::{
    finalization::{authorization, finalization},
    shared::{Shared, lock},
};
use sil_crypto::sha256;
use sil_ledger::EventBody;
use sil_types::{JobId, PublicKey};
use std::{sync::Arc, time::Duration};
use tokio::{sync::watch, time::interval};

pub(crate) async fn run(shared: Arc<Shared>, tick: Duration, mut stop: watch::Receiver<bool>) {
    let mut timer = interval(tick);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = timer.tick() => {
                let shared = Arc::clone(&shared);
                // Dérivation d'état : calcul, hors de la boucle asynchrone.
                let _ = tokio::task::spawn_blocking(move || {
                    for job in shared.job_ids() {
                        finalize(&shared, &job);
                        queue_settlements(&shared, &job);
                        crate::reputation_feed::observe_outcomes(&shared, &job);
                    }
                })
                .await;
            }
        }
    }
}

/// Pas entre deux rangs : le premier approbateur publie tout de suite, les suivants attendent leur
/// tour. Un délai d'un rang ne retarde donc jamais le premier.
const FINALIZE_STEP_MS: u64 = 3_000;
/// Au-delà, une entrée de rang est oubliée (un fragment déjà finalisé par un autre n'y revient pas).
const READY_TTL_MS: u64 = 3_600_000;

/// Publie le `Finalize` de chaque fragment dont le résultat est approuvé (ADR 0005, phase 3).
/// Chaque tentative a un rang déterministe parmi ses approbateurs ; le rang `n` attend `n` pas.
fn finalize(shared: &Shared, job: &JobId) {
    let now = shared.now_ms();
    let fragments = match lock(&shared.ledger).spec(job) {
        Some(spec) => spec.spec.fragments,
        None => return,
    };
    let me = shared.author();
    for fragment in 0..fragments {
        let Some(body) = finalization(&lock(&shared.ledger), job, fragment, now) else {
            continue;
        };
        // Le rang se calcule sur les approbateurs du certificat, quel que soit le mode.
        let (attempt, approvers): (u32, Vec<PublicKey>) = match &body {
            EventBody::Finalize {
                attempt,
                certificate,
                ..
            } => (
                *attempt,
                certificate.approvals.iter().map(|a| a.verifier).collect(),
            ),
            EventBody::FinalizeTolerance {
                attempt,
                certificate,
                ..
            } => (
                *attempt,
                certificate.approvals.iter().map(|a| a.verifier).collect(),
            ),
            _ => continue,
        };
        let rank = finalize_rank(job, fragment, attempt, &approvers, &me);
        let key = (*job, fragment, attempt);
        let ready_at = {
            let mut work = lock(&shared.work);
            work.finalize_ready
                .retain(|_, at| at.saturating_add(READY_TTL_MS) > now);
            *work
                .finalize_ready
                .entry(key)
                .or_insert(now + rank as u64 * FINALIZE_STEP_MS)
        };
        if now < ready_at {
            continue;
        }
        if let Err(error) = shared.author_event(job, body) {
            tracing::debug!(%error, "finalisation non enregistrée");
        }
    }
}

/// Rang de `me` parmi les approbateurs d'une tentative : ordre fixé par un hash de
/// (job, fragment, tentative, clé), donc le même pour tous les nœuds. Un nœud qui n'a pas approuvé
/// passe après tous les approbateurs.
pub(crate) fn finalize_rank(
    job: &JobId,
    fragment: u32,
    attempt: u32,
    approvers: &[PublicKey],
    me: &PublicKey,
) -> usize {
    let mut order: Vec<(Vec<u8>, PublicKey)> = approvers
        .iter()
        .map(|key| {
            let mut seed = job.to_hex().into_bytes();
            seed.extend_from_slice(&fragment.to_le_bytes());
            seed.extend_from_slice(&attempt.to_le_bytes());
            seed.extend_from_slice(key.as_bytes());
            (sha256(&seed).as_bytes().to_vec(), *key)
        })
        .collect();
    order.sort();
    order
        .iter()
        .position(|(_, key)| key == me)
        .unwrap_or(order.len())
}

/// Met en file le règlement de chaque fragment finalisé, avec les signatures des approbateurs.
fn queue_settlements(shared: &Shared, job: &JobId) {
    let now = shared.now_ms();
    let fragments = match lock(&shared.ledger).spec(job) {
        Some(spec) => spec.spec.fragments,
        None => return,
    };
    for fragment in 0..fragments {
        let auth = authorization(&lock(&shared.ledger), job, fragment, now);
        if let Some(auth) = auth {
            let _ = lock(&shared.outbox).enqueue(auth);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> PublicKey {
        PublicKey::from_bytes([byte; 32])
    }

    #[test]
    fn approvers_get_distinct_ranks_and_outsiders_go_last() {
        let job = JobId::from_bytes([7; 32]);
        let approvers = [key(1), key(2), key(3)];
        let mut ranks: Vec<usize> = approvers
            .iter()
            .map(|me| finalize_rank(&job, 0, 1, &approvers, me))
            .collect();
        ranks.sort_unstable();
        assert_eq!(ranks, vec![0, 1, 2]);
        assert_eq!(finalize_rank(&job, 0, 1, &approvers, &key(9)), 3);
    }
}
