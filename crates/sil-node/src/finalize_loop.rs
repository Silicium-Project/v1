//! Finaliseur (ADR 0005 phase 4, ADR 0007) : tout nœud publie le `Finalize` d'un résultat approuvé,
//! et met en file le règlement signé par les approbateurs. Il n'y a ni comité ni vote.

use crate::{
    finalization::{authorization, finalization},
    shared::{Shared, lock},
};
use sil_types::JobId;
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

/// Publie le `Finalize` de chaque fragment dont le résultat est approuvé (ADR 0005, phase 3).
fn finalize(shared: &Shared, job: &JobId) {
    let now = shared.now_ms();
    let fragments = match lock(&shared.ledger).spec(job) {
        Some(spec) => spec.spec.fragments,
        None => return,
    };
    for fragment in 0..fragments {
        let body = finalization(&lock(&shared.ledger), job, fragment, now);
        if let Some(body) = body
            && let Err(error) = shared.author_event(job, body)
        {
            tracing::debug!(%error, "finalisation non enregistrée");
        }
    }
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
