//! Branche l'API de contrôle sur le nœud.

use crate::blob::ResultBlob;
use crate::fetch::ensure_content;
use crate::input::read_all;
use crate::shared::{Shared, lock};
use serde_json::{Value, json};
use sil_control_api::{ApiError, Backend, EventReceipt, FragmentOutput, JobImage};
use sil_ledger::{Event, JobSpec, LedgerError};
use sil_types::{ContentId, JobId};
use std::sync::Arc;

/// Taille maximale d'un événement accepté par l'API.
const MAX_EVENT_BYTES: usize = 64 * 1024;
/// Taille maximale d'un résultat de fragment lu depuis le magasin.
const MAX_RESULT_BYTES: usize = 256 * 1024 * 1024;
/// Taille maximale d'un résultat de fragment donné par l'API en hexadécimal.
const MAX_OUTPUT_BYTES: usize = 4096;

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
        let (state, workload, head) = {
            let ledger = lock(&self.shared.ledger);
            let state = ledger.job_state(job, self.shared.now_ms());
            let spec = ledger.spec(job);
            let workload = spec.map(|signed| signed.spec.workload.clone());
            // Tête de la chaîne du requester : le web en tire la séquence et le `previous` de son
            // événement `Cancel`.
            let head = spec.and_then(|signed| ledger.head(job, &signed.spec.requester));
            (state, workload, head)
        };
        state
            .map(|state| {
                let mut view = serde_json::to_value(state)
                    .map_err(|e| ApiError::Unavailable(e.to_string()))?;
                // The workload tells the API whether the job produces an image.
                if let (Value::Object(fields), Some(workload)) = (&mut view, workload) {
                    let image = workload == sil_workload_spec::RENDU_RAYTRACER.id;
                    fields.insert("workload".into(), Value::String(workload));
                    fields.insert("image".into(), Value::Bool(image));
                }
                if let (Value::Object(fields), Some((sequence, hash))) = (&mut view, head) {
                    fields.insert(
                        "requester_head".into(),
                        serde_json::json!({ "sequence": sequence, "hash": hash.to_hex() }),
                    );
                }
                Ok(view)
            })
            .transpose()
    }

    fn job_image(&self, job: &JobId) -> Result<JobImage, ApiError> {
        let (spec, bands) = self.finished_results(job)?;
        if spec.workload != sil_workload_spec::RENDU_RAYTRACER.id {
            return Err(bad("ce workload ne produit pas d'image"));
        }
        let rgb = join_rows(bands, spec.width, spec.height)?;
        Ok(JobImage {
            width: spec.width,
            height: spec.height,
            rgb,
        })
    }

    fn job_outputs(&self, job: &JobId) -> Result<Vec<FragmentOutput>, ApiError> {
        let (_, results) = self.finished_results(job)?;
        results
            .into_iter()
            .enumerate()
            .map(|(index, bytes)| {
                if bytes.len() > MAX_OUTPUT_BYTES {
                    return Err(bad("résultat trop grand pour l'API : utiliser l'image"));
                }
                let fragment = u32::try_from(index)
                    .map_err(|_| ApiError::Unavailable("trop de fragments".into()))?;
                Ok(FragmentOutput { fragment, bytes })
            })
            .collect()
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

    fn submit_settlement(&self, bytes: Vec<u8>) -> Result<Value, ApiError> {
        if !self.shared.chain.pays_fees() {
            return Err(ApiError::Unavailable(
                "ce nœud ne paie pas les frais de règlement".into(),
            ));
        }
        let auth: sil_settlement::Authorization = sil_codec::decode(&bytes, MAX_EVENT_BYTES)
            .map_err(|_| ApiError::BadRequest("règlement illisible".into()))?;
        if auth.approver_signers().is_empty() {
            return Err(ApiError::BadRequest("aucune approbation valide".into()));
        }
        let escrow = self
            .shared
            .chain
            .escrow(&auth.claim.job)
            .map_err(|e| ApiError::Unavailable(e.to_string()))?
            .ok_or(ApiError::NotFound)?;
        if escrow.settled(auth.claim.fragment).is_some() {
            return Err(ApiError::BadRequest("fragment déjà réglé".into()));
        }
        let tx = self.shared.chain.submit(&auth).map_err(|e| match e {
            sil_settlement::ChainError::Rejected(text) => ApiError::BadRequest(text),
            other => ApiError::Unavailable(other.to_string()),
        })?;
        Ok(json!({ "tx": tx.0 }))
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
    ]
    .into_iter()
    .filter(|(bit, _)| mask & bit != 0)
    .map(|(_, name)| name)
    .collect()
}

/// Concatène les bandes de lignes, dans l'ordre des fragments, et vérifie la taille de l'image.
fn join_rows(bands: Vec<Vec<u8>>, width: u32, height: u32) -> Result<Vec<u8>, ApiError> {
    let expected = u64::from(width) * u64::from(height) * 3;
    let rgb = bands.concat();
    if rgb.len() as u64 != expected {
        return Err(ApiError::Unavailable(format!(
            "taille d'image inattendue : {} octets au lieu de {expected}",
            rgb.len()
        )));
    }
    Ok(rgb)
}

impl NodeBackend {
    /// Spec du job et résultat brut de chaque fragment, dans l'ordre. Refusé tant qu'un fragment
    /// n'est pas final ou que son contenu n'est pas disponible.
    fn finished_results(&self, job: &JobId) -> Result<(JobSpec, Vec<Vec<u8>>), ApiError> {
        let now = self.shared.now_ms();
        let (spec, states) = {
            let ledger = lock(&self.shared.ledger);
            let spec = ledger.spec(job).ok_or(ApiError::NotFound)?.spec.clone();
            let states: Vec<_> = (0..spec.fragments)
                .map(|fragment| ledger.task_state(job, fragment, now))
                .collect();
            (spec, states)
        };
        let mut results = Vec::with_capacity(states.len());
        for (fragment, state) in states.into_iter().enumerate() {
            let result = state
                .and_then(|state| state.finalized)
                .map(|finalized| finalized.result)
                .ok_or_else(|| {
                    ApiError::Unavailable(format!("fragment {fragment} pas encore final"))
                })?;
            let id = ContentId::from_bytes(*result.as_bytes());
            if !self
                .shared
                .runtime
                .block_on(ensure_content(&self.shared, &id))
            {
                return Err(ApiError::Unavailable(format!(
                    "fragment {fragment} : contenu indisponible"
                )));
            }
            let raw = read_all(&self.shared.store, &id)
                .map_err(|e| ApiError::Unavailable(e.to_string()))?;
            let blob = ResultBlob::from_bytes(&raw, MAX_RESULT_BYTES)
                .map_err(|e| ApiError::Unavailable(e.to_string()))?;
            results.push(blob.into_output().result);
        }
        Ok((spec, results))
    }
}

#[cfg(test)]
mod tests {
    use super::join_rows;

    #[test]
    fn bands_join_in_fragment_order_and_the_size_must_match() {
        assert_eq!(
            join_rows(vec![vec![1, 2, 3], vec![4, 5, 6]], 1, 2).map_err(|_| ()),
            Ok(vec![1, 2, 3, 4, 5, 6])
        );
        assert!(join_rows(vec![vec![1, 2, 3]], 1, 2).is_err());
    }
}
