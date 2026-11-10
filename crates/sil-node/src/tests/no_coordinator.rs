//! Phase 4 (ADR 0005) : trois nœuds sans rôle coordinateur mènent un job à son terme. Aucun vote
//! du comité n'est émis : le claim, les vérificateurs et la finalisation passent par le journal.

use super::{client::*, scenario::*, support::*};
use sil_settlement::testing::FakeChain;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn three_nodes_without_coordinator_finish_a_job() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let spec = job_spec(input_hash(&data));
    chain.insert_escrow(escrow_for(&spec));
    let a = start(1, roles(true, true), sha_registry(), chain.clone(), true).await;
    let b = start(2, roles(true, true), sha_registry(), chain.clone(), false).await;
    let c = start(3, roles(true, true), sha_registry(), chain.clone(), false).await;
    let nodes = [&a, &b, &c];
    connect_all(&nodes).await;
    let miner = spawn_miner(chain.clone());
    upload_input(&a, &data).await;
    let job = publish_job(&a, spec).await;
    wait_dump(90, "job finalisé sans coordinateur", &nodes, &job, || {
        finalized_everywhere(&nodes, &job)
    })
    .await;
    miner.abort();
    for node in [a, b, c] {
        node.node.shutdown().await;
    }
}
