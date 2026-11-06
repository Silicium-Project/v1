//! Membre du comité : émet les votes décidés par `sil-coordinator`, puis signe les règlements
//! des fragments finalisés.

use crate::{
    shared::{Shared, lock},
    wire::Envelope,
};
use sil_coordinator::Coordinator;
use sil_net::Topic;
use sil_settlement::{Authorization, SettlementClaim};
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
                // Dérivation d'état et signatures : calcul, hors de la boucle asynchrone.
                let _ = tokio::task::spawn_blocking(move || {
                    for job in shared.job_ids() {
                        vote(&shared, &coordinator_for(&shared), &job);
                        sign_settlements(&shared, &job);
                        crate::reputation_feed::observe_outcomes(&shared, &job);
                    }
                })
                .await;
            }
        }
    }
}

/// Un automate neuf (il est sans état : tout vient du journal).
fn coordinator_for(shared: &Shared) -> Coordinator {
    Coordinator::with_window(shared.fresh_identity(), shared.claim_window_ms)
        .with_leader_wait(shared.leader_wait_ms)
}

fn vote(shared: &Shared, coordinator: &Coordinator, job: &JobId) {
    let now = shared.now_ms();
    let votes = {
        let ledger = lock(&shared.ledger);
        let reputation = lock(&shared.reputation);
        coordinator.decide(&ledger, &reputation, job, now)
    };
    for body in votes {
        if let Err(error) = shared.author_event(job, body) {
            tracing::debug!(%error, "vote non enregistré");
        }
    }
}

/// Pour chaque fragment finalisé : signe le règlement, le met en file et diffuse la signature.
fn sign_settlements(shared: &Shared, job: &JobId) {
    let now = shared.now_ms();
    let me = shared.identity.public();
    let (required, fragments) = {
        let ledger = lock(&shared.ledger);
        match ledger.spec(job) {
            Some(spec) if spec.spec.committee.contains(&me) => {
                (spec.spec.verifications_required, spec.spec.fragments)
            }
            _ => return,
        }
    };
    for fragment in 0..fragments {
        let state = lock(&shared.ledger).task_state(job, fragment, now);
        let Some(finalized) = state.and_then(|s| s.finalized) else {
            continue;
        };
        let Some(claim) = SettlementClaim::from_finalized(job, fragment, &finalized, required)
        else {
            continue;
        };
        let id = claim.id();
        if !lock(&shared.work).signed.insert(id) {
            continue;
        }
        let signature = claim.sign(&shared.identity);
        let mut authorization = Authorization::new(claim.clone());
        if authorization.add(me, signature).is_ok() {
            let _ = lock(&shared.outbox).enqueue(authorization);
        }
        shared.broadcast(
            &Envelope::SettlementSig {
                claim: Box::new(claim),
                signer: me,
                signature,
            },
            &[Topic::for_job(job)],
        );
    }
}
