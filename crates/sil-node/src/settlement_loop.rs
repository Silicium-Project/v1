//! Fait avancer l'outbox de règlement et la sauvegarde sur disque.

use crate::shared::{Shared, lock};
use std::{sync::Arc, time::Duration};
use tokio::{sync::watch, time::interval};

pub(crate) async fn run(shared: Arc<Shared>, mut stop: watch::Receiver<bool>) {
    let mut timer = interval(Duration::from_secs(1));
    let mut saved_len = usize::MAX;
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = timer.tick() => {
                let work = Arc::clone(&shared);
                let previous = saved_len;
                // Appels de chaîne bloquants : hors de la boucle asynchrone.
                if let Ok(len) = tokio::task::spawn_blocking(move || step(&work, previous)).await {
                    saved_len = len;
                }
            }
        }
    }
    // Dernière sauvegarde à l'arrêt.
    let outbox = lock(&shared.outbox);
    if let Err(error) = shared.outbox_file.save(&outbox) {
        tracing::error!(%error, "outbox non sauvegardée à l'arrêt");
    }
}

/// Un tour : avance les règlements, sauvegarde si quelque chose a changé.
fn step(shared: &Shared, previous_len: usize) -> usize {
    let now = shared.now_ms();
    let mut outbox = lock(&shared.outbox);
    let changed = outbox.advance(&*shared.chain, now);
    if (changed > 0 || outbox.len() != previous_len)
        && let Err(error) = shared.outbox_file.save(&outbox)
    {
        tracing::error!(%error, "outbox non sauvegardée");
    }
    outbox.len()
}
