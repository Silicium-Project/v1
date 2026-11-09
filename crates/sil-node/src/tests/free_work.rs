use super::{client::*, scenario::*, support::*};
use crate::{ControlOptions, Node, NodeOptions, Roles};
use sil_crypto::MasterSeed;
use sil_settlement::testing::FakeChain;
use sil_types::JobId;
use sil_workload_api::Registry;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

/// Nœud avec API ; `tune` règle les options (modes de test).
pub(super) async fn start_tuned(
    seed: u8,
    roles: Roles,
    registry: Registry,
    chain: Arc<FakeChain>,
    tune: impl FnOnce(&mut NodeOptions),
) -> TestNode {
    let dir = tempfile::tempdir().unwrap();
    let mut options = NodeOptions::new(dir.path());
    options.roles = roles;
    tune(&mut options);
    options.tick_ms = 100;
    options.trusted_requesters = vec![ledger_key(100)];
    options.listen = vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()];
    options.control = Some(ControlOptions {
        bind: "127.0.0.1:0".parse().unwrap(),
        token_file: dir.path().join("control.token"),
    });
    let node = Node::start(options, MasterSeed::from_bytes([seed; 32]), registry, chain)
        .await
        .unwrap();
    let api = node.control_addr();
    let peer = node.net().local_peer_id();
    TestNode {
        node,
        dir,
        api,
        key: ledger_key(seed),
        peer,
    }
}

/// Vrai si le nœud tient une tentative du job : son claim est le premier valide (phase 2).
fn claimed(node: &TestNode, job: &JobId) -> bool {
    let me = node.key;
    let ledger = crate::shared::lock(&node.node.shared().ledger);
    (0..2).any(|fragment| {
        ledger
            .task_state(job, fragment, now_ms())
            .is_some_and(|state| state.attempts.iter().any(|attempt| attempt.claimant == me))
    })
}

/// Sans escrow ni paiement, un worker en mode test réclame quand même son fragment.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_worker_claims_without_escrow_in_free_work_mode() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let spec = job_spec(input_hash(&data));
    let node = start_tuned(1, roles(true, false), sha_registry(), chain, |o| {
        o.free_work = true
    })
    .await;
    upload_input(&node, &data).await;
    let job = publish_job(&node, spec).await;
    wait_dump(60, "fragment réclamé sans escrow", &[&node], &job, || {
        claimed(&node, &job)
    })
    .await;
}

/// Hors mode test, sans escrow, le même worker ne réclame rien.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_free_work_a_worker_waits_for_an_escrow() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let spec = job_spec(input_hash(&data));
    let node = start(1, roles(true, false), sha_registry(), chain, true).await;
    upload_input(&node, &data).await;
    let job = publish_job(&node, spec).await;
    sleep(Duration::from_secs(3)).await;
    assert!(
        !claimed(&node, &job),
        "réclamation sans escrow hors mode test"
    );
}
