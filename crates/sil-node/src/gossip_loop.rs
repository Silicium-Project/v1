//! Réception des messages, rattrapage des trous, rediffusion.

use crate::{
    shared::{Shared, lock},
    wire::Envelope,
};
use sil_ledger::Event;
use sil_net::{LedgerRequest, NetEvent, Topic};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, watch},
    time::interval,
};

const GAP_EVERY: Duration = Duration::from_secs(2);
const REBROADCAST_EVERY: Duration = Duration::from_secs(8);
const MAX_GAPS_PER_ROUND: usize = 8;
const MAX_PEERS_TRIED: usize = 4;
const SYNC_EVERY: Duration = Duration::from_secs(2);
/// Délai avant de redemander à un pair à jour s'il a du nouveau.
const RESYNC_MS: u64 = 15_000;
const MAX_SYNC_JOBS: usize = 32;
const MAX_SYNC_PEERS: usize = 2;
const SYNC_PAGE: u16 = 64;

pub(crate) async fn run(
    shared: Arc<Shared>,
    mut events: mpsc::Receiver<NetEvent>,
    mut stop: watch::Receiver<bool>,
) {
    let mut gaps = interval(GAP_EVERY);
    let mut rebroadcast = interval(REBROADCAST_EVERY);
    let mut sync = interval(SYNC_EVERY);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            event = events.recv() => match event {
                Some(NetEvent::Gossip { from, topic, data }) => {
                    if topic == Topic::Presence {
                        crate::presence_loop::on_presence(&shared, from, &data);
                    } else {
                        on_message(&shared, &data);
                    }
                }
                Some(_) => {}
                None => break,
            },
            _ = gaps.tick() => fill_gaps(&shared).await,
            _ = sync.tick() => sync_jobs(&shared).await,
            _ = rebroadcast.tick() => rebroadcast_heads(&shared),
        }
    }
}

fn on_message(shared: &Arc<Shared>, data: &[u8]) {
    match Envelope::from_bytes(data) {
        Ok(Envelope::Event(event)) => {
            if let Err(error) = shared.ingest_live(*event) {
                tracing::debug!(%error, "événement reçu non retenu");
            }
        }
        Err(_) => {}
    }
}

/// Demande aux pairs les événements manquants (point à point, borné).
async fn fill_gaps(shared: &Arc<Shared>) {
    let gaps = lock(&shared.ledger).gaps();
    let Some(net) = shared.net.get() else {
        return;
    };
    if gaps.is_empty() {
        return;
    }
    let peers = net.peers().await.unwrap_or_default();
    for gap in gaps.into_iter().take(MAX_GAPS_PER_ROUND) {
        let request = LedgerRequest::Author {
            job: gap.job,
            author: gap.author,
            after_sequence: gap.after_sequence,
            max: 64,
        };
        for peer in peers.iter().take(MAX_PEERS_TRIED) {
            let Ok(response) = net.request_ledger(*peer, request.clone()).await else {
                continue;
            };
            let events = response.into_events().unwrap_or_default();
            if events.is_empty() {
                continue;
            }
            for bytes in events {
                if let Ok(event) = Event::from_bytes(&bytes, 64 * 1024) {
                    let _ = shared.ingest(event);
                }
            }
            break;
        }
    }
}

/// Rattrape l'historique des jobs connus auprès de quelques pairs, page par page, par curseur.
/// C'est ce qui permet à un nœud arrivé tard (ou qui a manqué des messages) de reconstruire
/// l'état, y compris les événements d'auteurs qu'il ne connaît pas encore.
async fn sync_jobs(shared: &Arc<Shared>) {
    let Some(net) = shared.net.get() else {
        return;
    };
    let peers = net.peers().await.unwrap_or_default();
    if peers.is_empty() {
        return;
    }
    let now = shared.now_ms();
    let jobs = lock(&shared.ledger).job_ids();
    for job in jobs.into_iter().take(MAX_SYNC_JOBS) {
        for peer in peers.iter().take(MAX_SYNC_PEERS) {
            let state = lock(&shared.work)
                .sync
                .get(&(job, *peer))
                .copied()
                .unwrap_or_default();
            if state
                .caught_up_at
                .is_some_and(|at| now.saturating_sub(at) < RESYNC_MS)
            {
                continue;
            }
            let request = LedgerRequest::Cursor {
                job,
                cursor: state.cursor,
                max: SYNC_PAGE,
            };
            let Ok(response) = net.request_ledger(*peer, request).await else {
                continue;
            };
            let Some(events) = response.into_events() else {
                continue; // pair saturé : plus tard
            };
            let received = events.len();
            for bytes in events {
                if let Ok(event) = Event::from_bytes(&bytes, 64 * 1024) {
                    let _ = shared.ingest(event);
                }
            }
            let mut work = lock(&shared.work);
            let entry = work.sync.entry((job, *peer)).or_default();
            entry.cursor += received as u64;
            entry.caught_up_at = (received < usize::from(SYNC_PAGE)).then_some(now);
        }
    }
}

/// Rediffuse la dernière entrée de nos chaînes (les autres détectent alors ce qui leur manque),
/// et l'annonce du job.
fn rebroadcast_heads(shared: &Arc<Shared>) {
    let me = shared.identity.public();
    let jobs = lock(&shared.ledger).job_ids();
    for job in jobs {
        let (mine, announcement) = {
            let ledger = lock(&shared.ledger);
            let mine = ledger.events_of(&job, &me).pop();
            let announcement = ledger.spec(&job).and_then(|spec| {
                ledger
                    .events_of(&job, &spec.spec.requester)
                    .into_iter()
                    .next()
            });
            (mine, announcement)
        };
        if let Some(event) = mine {
            shared.publish_event(&event);
        }
        if let Some(event) = announcement {
            shared.broadcast(&Envelope::Event(Box::new(event)), &[Topic::Announce]);
        }
    }
}
