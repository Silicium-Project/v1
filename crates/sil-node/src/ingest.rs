//! Entrée et sortie des événements : réception, persistance, publication.

use crate::{
    shared::{Shared, lock},
    wire::Envelope,
};
use sil_ledger::{Event, EventBody, LedgerError, Receipt};
use sil_net::Topic;
use sil_types::JobId;

impl Shared {
    /// Reçoit un événement (réseau ou API), le persiste s'il est nouveau.
    pub(crate) fn ingest(&self, event: Event) -> Result<Receipt, LedgerError> {
        let receipt = lock(&self.ledger).receive(event)?;
        self.persist(&receipt.accepted);
        for accepted in &receipt.accepted {
            if matches!(accepted.body, EventBody::JobPublished { .. }) {
                self.follow_job(&accepted.job);
            }
        }
        Ok(receipt)
    }

    fn persist(&self, events: &[Event]) {
        if events.is_empty() {
            return;
        }
        let mut log = lock(&self.log);
        for event in events {
            if let Err(error) = log.append(event) {
                tracing::error!(%error, "événement accepté mais non persisté");
            }
        }
    }

    /// Signe, enregistre et diffuse un événement de ce nœud.
    pub(crate) fn author_event(&self, job: &JobId, body: EventBody) -> Result<Event, LedgerError> {
        let now = self.now_ms();
        let event = lock(&self.ledger).append_local(&self.identity, *job, body, now)?;
        self.persist(std::slice::from_ref(&event));
        self.publish_event(&event);
        Ok(event)
    }

    /// Diffuse un événement : fragment du job, et `announce` pour une publication de job.
    pub(crate) fn publish_event(&self, event: &Event) {
        let mut topics = vec![Topic::for_job(&event.job)];
        if matches!(event.body, EventBody::JobPublished { .. }) {
            topics.push(Topic::Announce);
        }
        self.broadcast(&Envelope::Event(Box::new(event.clone())), &topics);
    }

    pub(crate) fn broadcast(&self, envelope: &Envelope, topics: &[Topic]) {
        let Some(net) = self.net.get().cloned() else {
            return;
        };
        let Ok(bytes) = envelope.to_bytes() else {
            return;
        };
        let topics = topics.to_vec();
        self.runtime.spawn(async move {
            for topic in topics {
                // Échec fréquent au démarrage (maillage non formé) : la rediffusion périodique
                // et le rattrapage des trous rattrapent.
                if let Err(error) = net.publish(topic, bytes.clone()).await {
                    tracing::debug!(%error, "publication gossip impossible");
                }
            }
        });
    }
}
