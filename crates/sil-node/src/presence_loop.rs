//! Présence : annonce périodique, minimale, des rôles et de la charge de ce nœud.

use crate::shared::{Shared, lock};
use sil_net::{PeerId, Presence, PresenceGate, ROLE_COMPUTE, ROLE_COORDINATOR, ROLE_VERIFY, Topic};
use std::{sync::Arc, sync::atomic::Ordering, time::Duration};
use tokio::{sync::watch, time::interval};

const EVERY: Duration = Duration::from_secs(2);
const MIN_GAP_MS: u64 = 2_000;
const MAX_SILENCE_MS: u64 = 30_000;
/// Présences conservées au plus, et durée après laquelle un pair silencieux est oublié.
const MAX_TRACKED: usize = 1024;
const FORGET_AFTER_MS: u64 = 180_000;
const MAX_LEN: usize = 512;

fn local_presence(shared: &Shared) -> Presence {
    let roles = &shared.roles;
    let mask = (if roles.worker { ROLE_COMPUTE } else { 0 })
        | (if roles.verifier { ROLE_VERIFY } else { 0 })
        | (if roles.coordinator {
            ROLE_COORDINATOR
        } else {
            0
        });
    let active = shared.active.load(Ordering::Relaxed);
    let mut workloads: Vec<String> = shared
        .registry
        .ids()
        .iter()
        .map(|id| (*id).to_string())
        .collect();
    workloads.truncate(8);
    Presence {
        version: 1,
        roles: mask,
        load: u8::try_from(active.saturating_mul(25).min(100)).unwrap_or(100),
        workloads,
    }
}

pub(crate) async fn run(shared: Arc<Shared>, mut stop: watch::Receiver<bool>) {
    let mut timer = interval(EVERY);
    let mut gate = PresenceGate::new(MIN_GAP_MS, MAX_SILENCE_MS);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = timer.tick() => {
                let presence = local_presence(&shared);
                let Some(net) = shared.net.get() else { continue };
                if !gate.should_send(&presence, shared.now_ms()) {
                    continue;
                }
                let Some(bytes) = presence.to_bytes() else { continue };
                if net.publish(Topic::Presence, bytes).await.is_err() {
                    // Maillage pas encore formé : on réessaiera au prochain tour.
                    gate = PresenceGate::new(MIN_GAP_MS, MAX_SILENCE_MS);
                }
            }
        }
    }
}

/// Retient la présence reçue d'un pair (table bornée, entrées anciennes oubliées).
pub(crate) fn on_presence(shared: &Shared, from: PeerId, data: &[u8]) {
    let Some(presence) = Presence::from_bytes(data, MAX_LEN) else {
        return;
    };
    let now = shared.now_ms();
    let mut work = lock(&shared.work);
    if work.presence.len() >= MAX_TRACKED && !work.presence.contains_key(&from) {
        work.presence
            .retain(|_, (_, seen)| now.saturating_sub(*seen) < FORGET_AFTER_MS);
        if work.presence.len() >= MAX_TRACKED {
            return;
        }
    }
    work.presence.insert(from, (presence, now));
}
