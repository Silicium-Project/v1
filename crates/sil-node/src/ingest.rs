//! Entrée et sortie des événements : réception, persistance, publication.

use crate::{
    shared::{Shared, lock},
    wire::Envelope,
};
use sil_ledger::{Event, EventBody, LedgerError, Receipt};
use sil_net::Topic;
use sil_types::JobId;

/// Dérive d'horloge admise à la réception (ADR 0005, règle 1) : un événement daté de plus de
/// cette durée dans le futur est refusé. Le journal ne lit l'heure que dans les événements.
pub(crate) const MAX_SKEW_MS: u64 = 30_000;

/// Âge maximal d'un événement reçu en direct (gossip, API). Un événement antidaté gagnerait la
/// course d'un `claim` (ordre `(issued_at_ms, id)`) : on le refuse s'il est trop ancien. Le
/// rattrapage (`sync_jobs`, `fill_gaps`) passe par `ingest` et garde l'historique ancien.
pub(crate) const MAX_LIVE_AGE_MS: u64 = 60_000;

/// Vrai si un événement reçu en direct, daté `issued_at_ms`, est plus vieux que `MAX_LIVE_AGE_MS`.
pub(crate) fn too_old_for_live(issued_at_ms: u64, now_ms: u64) -> bool {
    issued_at_ms.saturating_add(MAX_LIVE_AGE_MS) < now_ms
}

/// Fenêtre d'ancrage d'un claim, en slots Solana (ADR 0009), définie par le journal.
pub(crate) use sil_ledger::CLAIM_WINDOW_SLOTS;

/// Tolérance sur le retard de l'RPC local : un slot ancré légèrement au-dessus du slot local est
/// accepté (deux nœuds ne lisent pas le même slot au même instant). Au-delà, le claim est refusé.
pub(crate) const CLAIM_LAG_TOLERANCE_SLOTS: u64 = 2;

/// Vérifie l'ancrage d'un claim reçu en direct : le slot doit être dans la fenêtre récente lue sur
/// la chaîne locale. Un nœud sans chaîne ne valide aucun claim (ADR 0009).
fn check_claim_anchor(anchor_slot: u64, current_slot: u64) -> Result<(), LedgerError> {
    if anchor_slot > current_slot.saturating_add(CLAIM_LAG_TOLERANCE_SLOTS) {
        return Err(LedgerError::InvalidEvent("claim ancré dans le futur"));
    }
    if anchor_slot.saturating_add(CLAIM_WINDOW_SLOTS) < current_slot {
        return Err(LedgerError::InvalidEvent("claim ancré hors fenêtre"));
    }
    Ok(())
}

/// Borne haute de l'ancrage d'un claim reçu par rattrapage : un slot au-delà du slot local, tolérance
/// comprise, n'existe pas encore. Pas de borne basse : l'historique ancien reste rejouable.
fn check_synced_anchor(anchor_slot: u64, current_slot: u64) -> Result<(), LedgerError> {
    if anchor_slot > current_slot.saturating_add(CLAIM_LAG_TOLERANCE_SLOTS) {
        return Err(LedgerError::InvalidEvent(
            "claim ancré dans le futur (rattrapage)",
        ));
    }
    Ok(())
}

impl Shared {
    /// Reçoit un événement par rattrapage : même règle que `ingest`, plus la borne haute sur
    /// l'ancrage d'un claim. Sans slot local, pas de claim (ADR 0009) : le trou se comble plus tard.
    pub(crate) fn ingest_synced(&self, event: Event) -> Result<Receipt, LedgerError> {
        if let EventBody::Claim { anchor_slot, .. } = event.body {
            let current = self.chain.current_slot().map_err(|_| {
                LedgerError::InvalidEvent("claim sans slot local (chaîne indisponible)")
            })?;
            check_synced_anchor(anchor_slot, current)?;
        }
        self.ingest(event)
    }

    /// Reçoit un événement reçu en direct (gossip ou API) : même règle que `ingest`, plus la borne
    /// sur l'âge, et pour un claim, la borne sur l'ancrage (ADR 0009).
    pub(crate) fn ingest_live(&self, event: Event) -> Result<Receipt, LedgerError> {
        if too_old_for_live(event.issued_at_ms, self.now_ms()) {
            return Err(LedgerError::InvalidEvent("événement daté dans le passé"));
        }
        if let EventBody::Claim { anchor_slot, .. } = event.body {
            let current = self
                .chain
                .current_slot()
                .map_err(|_| LedgerError::InvalidEvent("claim sans slot local (chaîne indisponible)"))?;
            check_claim_anchor(anchor_slot, current)?;
        }
        self.ingest(event)
    }

    /// Reçoit un événement (réseau ou rattrapage), le persiste s'il est nouveau.
    pub(crate) fn ingest(&self, event: Event) -> Result<Receipt, LedgerError> {
        if event.issued_at_ms > self.now_ms().saturating_add(MAX_SKEW_MS) {
            return Err(LedgerError::InvalidEvent("événement daté dans le futur"));
        }
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

#[cfg(test)]
mod anchor_tests {
    use super::*;

    #[test]
    fn an_anchor_inside_the_window_is_accepted() {
        assert!(check_claim_anchor(1_000, 1_000).is_ok());
        assert!(check_claim_anchor(1_000 - CLAIM_WINDOW_SLOTS, 1_000).is_ok());
    }

    #[test]
    fn an_anchor_older_than_the_window_is_refused() {
        assert!(check_claim_anchor(1_000 - CLAIM_WINDOW_SLOTS - 1, 1_000).is_err());
    }

    #[test]
    fn a_small_lag_of_the_local_rpc_is_tolerated_but_not_a_large_one() {
        assert!(check_claim_anchor(1_000 + CLAIM_LAG_TOLERANCE_SLOTS, 1_000).is_ok());
        assert!(check_claim_anchor(1_000 + CLAIM_LAG_TOLERANCE_SLOTS + 1, 1_000).is_err());
    }

    #[test]
    fn a_synced_claim_anchored_in_the_future_is_refused() {
        assert!(check_synced_anchor(1_000 + CLAIM_LAG_TOLERANCE_SLOTS + 1, 1_000).is_err());
        assert!(check_synced_anchor(u64::MAX, 1_000).is_err());
    }

    #[test]
    fn a_synced_claim_from_old_history_is_kept() {
        assert!(check_synced_anchor(1, 1_000_000).is_ok());
        assert!(check_synced_anchor(1_000 + CLAIM_LAG_TOLERANCE_SLOTS, 1_000).is_ok());
    }
}
