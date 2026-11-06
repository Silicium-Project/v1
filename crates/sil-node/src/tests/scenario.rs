use super::{client::http, support::*};
use crate::{blob::ResultBlob, input::read_all};
use sil_ledger::JobStatus;
use sil_net::Bytes;
use sil_types::{ContentId, JobId, PublicKey};
use sil_wl_sha256::Sha256Workload;
use sil_workload_api::{MemoryInput, Workload};

pub fn expected_digest(data: &[u8], fragment: usize) -> Vec<u8> {
    let input = MemoryInput(data);
    let specs = Sha256Workload.split(&input, 2).unwrap();
    Sha256Workload
        .execute(&input, &specs[fragment])
        .unwrap()
        .result
}

pub fn finalized_everywhere(nodes: &[&TestNode], job: &JobId) -> bool {
    nodes.iter().all(|n| {
        n.node
            .shared()
            .job_state(job)
            .is_some_and(|s| s.status == JobStatus::Finalized)
    })
}

/// Résultat stocké (sur le nœud qui l'a calculé) pour un fragment finalisé.
pub fn stored_result(nodes: &[&TestNode], job: &JobId, fragment: u32) -> (PublicKey, Vec<u8>) {
    let state = nodes[0].node.shared().job_state(job).unwrap();
    let finalized = state.tasks[fragment as usize].finalized.clone().unwrap();
    let id = ContentId::from_bytes(*finalized.result.as_bytes());
    for node in nodes {
        if node.node.shared().store().has(&id) {
            let bytes = read_all(node.node.shared().store(), &id).unwrap();
            let blob = ResultBlob::from_bytes(&bytes, 1 << 20).unwrap();
            let Bytes(result) = blob.result;
            return (finalized.claimant, result);
        }
    }
    panic!("résultat introuvable dans les magasins");
}

pub fn committee_keys() -> Vec<PublicKey> {
    vec![ledger_key(1), ledger_key(2), ledger_key(3)]
}

pub async fn upload_input(node: &TestNode, data: &[u8]) {
    let (status, _) = http(
        node.api.unwrap(),
        "PUT",
        "/v1/content",
        Some(&node.token()),
        data,
    )
    .await;
    assert_eq!(status, 201);
}
