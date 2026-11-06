//! Branche l'API de contrôle sur le nœud.

use crate::shared::{Shared, lock};
use serde_json::{Value, json};
use sil_control_api::{ApiError, Backend, EventReceipt};
use sil_ledger::{Event, LedgerError};
use sil_types::{ContentId, JobId};
use std::sync::Arc;

/// Taille maximale d'un événement accepté par l'API.
const MAX_EVENT_BYTES: usize = 64 * 1024;

pub struct NodeBackend {
    shared: Arc<Shared>,
}

impl NodeBackend {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

fn bad(reason: impl ToString) -> ApiError {
    ApiError::BadRequest(reason.to_string())
}

impl Backend for NodeBackend {
    fn put_content(&self, bytes: Vec<u8>) -> Result<ContentId, ApiError> {
        let id = self
            .shared
            .store
            .put_bytes(&bytes)
            .map_err(|e| ApiError::Unavailable(e.to_string()))?;
        if let Some(net) = self.shared.net.get().cloned() {
            self.shared.runtime.spawn(async move {
                let _ = net.provide(id).await;
            });
        }
        Ok(id)
    }

    fn submit_event(&self, bytes: Vec<u8>) -> Result<EventReceipt, ApiError> {
        let event = Event::from_bytes(&bytes, MAX_EVENT_BYTES).map_err(bad)?;
        let receipt = self.shared.ingest(event).map_err(|e| match e {
            LedgerError::TooManyJobs
            | LedgerError::JobFull
            | LedgerError::JobBytesFull
            | LedgerError::AuthorQuota => ApiError::Unavailable(e.to_string()),
            other => bad(other),
        })?;
        for event in &receipt.accepted {
            self.shared.publish_event(event);
        }
        Ok(EventReceipt {
            accepted: receipt.accepted.len(),
            pending: receipt.pending,
        })
    }

    fn job(&self, job: &JobId) -> Result<Option<Value>, ApiError> {
        let state = lock(&self.shared.ledger).job_state(job, self.shared.now_ms());
        state
            .map(|state| {
                serde_json::to_value(state).map_err(|e| ApiError::Unavailable(e.to_string()))
            })
            .transpose()
    }

    fn peers(&self) -> Result<Value, ApiError> {
        let Some(net) = self.shared.net.get() else {
            return Ok(json!([]));
        };
        let connected = self
            .shared
            .runtime
            .block_on(net.peers())
            .map_err(|e| ApiError::Unavailable(e.to_string()))?;
        let now = self.shared.now_ms();
        let work = lock(&self.shared.work);
        let peers: Vec<Value> = connected
            .iter()
            .map(|peer| match work.presence.get(peer) {
                Some((presence, seen)) => json!({
                    "peer": peer.to_string(),
                    "roles": role_names(presence.roles),
                    "load": presence.load,
                    "workloads": presence.workloads,
                    "seen_ms_ago": now.saturating_sub(*seen),
                }),
                None => json!({ "peer": peer.to_string() }),
            })
            .collect();
        Ok(json!(peers))
    }

    fn worker(&self) -> Result<Value, ApiError> {
        Ok(crate::worker_data::report(&self.shared))
    }

    fn settlements(&self) -> Result<Value, ApiError> {
        let report = self.shared.settlement_report();
        Ok(json!(
            report
                .into_iter()
                .map(|(id, status)| json!({ "id": id.to_hex(), "status": format!("{status:?}") }))
                .collect::<Vec<_>>()
        ))
    }
}

fn role_names(mask: u8) -> Vec<&'static str> {
    [
        (sil_net::ROLE_COMPUTE, "compute"),
        (sil_net::ROLE_VERIFY, "verify"),
        (sil_net::ROLE_STORAGE, "storage"),
        (sil_net::ROLE_COORDINATOR, "coordinator"),
    ]
    .into_iter()
    .filter(|(bit, _)| mask & bit != 0)
    .map(|(_, name)| name)
    .collect()
}
