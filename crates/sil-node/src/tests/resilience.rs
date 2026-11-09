use super::{client::*, scenario::*, slow::slow_registry, support::*};
use sil_settlement::testing::FakeChain;
use std::sync::Arc;

/// Vrai si les deux nœuds ont reçu les mêmes événements du job (mêmes têtes de chaîne).
fn same_journal(a: &TestNode, b: &TestNode, job: &sil_types::JobId) -> bool {
    let heads = |node: &TestNode| crate::shared::lock(&node.node.shared().ledger).heads(job);
    heads(a) == heads(b)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_long_computation_keeps_its_lease_alive() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let mut spec = job_spec(input_hash(&data));
    spec.lease_ms = 1_500; // le calcul dure 3 s : sans renouvellement le bail expire
    chain.insert_escrow(escrow_for(&spec));

    let c1 = start(
        1,
        roles(false, false),
        sha_registry(),
        chain.clone(),
        true,
    )
    .await;
    let c2 = start(
        2,
        roles(false, false),
        sha_registry(),
        chain.clone(),
        false,
    )
    .await;
    let c3 = start(
        3,
        roles(false, false),
        sha_registry(),
        chain.clone(),
        false,
    )
    .await;
    let slow = slow_registry(std::time::Duration::from_secs(3));
    let worker = start(5, roles(true, false), slow, chain.clone(), false).await;
    let v = start(
        4,
        roles(false, true),
        sha_registry(),
        chain.clone(),
        false,
    )
    .await;
    let nodes = [&c1, &c2, &c3, &worker, &v];
    connect_all(&nodes).await;
    let miner = spawn_miner(chain.clone());

    upload_input(&c1, &data).await;
    let job = publish_job(&c1, spec).await;
    wait_dump(90, "job finalisé", &nodes, &job, || {
        finalized_everywhere(&nodes, &job)
    })
    .await;

    let state = c1.node.shared().job_state(&job).unwrap();
    for task in &state.tasks {
        assert_eq!(
            task.attempts.len(),
            1,
            "une seule tentative : le bail a tenu"
        );
        assert!(
            task.attempts[0].renewals >= 1,
            "le worker a prolongé son bail ({} renouvellements)",
            task.attempts[0].renewals
        );
    }
    miner.abort();
    for node in [c1, c2, c3, worker, v] {
        node.node.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_node_joining_late_catches_up_through_sync() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let spec = job_spec(input_hash(&data));
    chain.insert_escrow(escrow_for(&spec));

    let c1 = start(
        1,
        roles(true, false),
        sha_registry(),
        chain.clone(),
        true,
    )
    .await;
    let c2 = start(
        2,
        roles(true, false),
        sha_registry(),
        chain.clone(),
        false,
    )
    .await;
    let c3 = start(
        3,
        roles(true, false),
        sha_registry(),
        chain.clone(),
        false,
    )
    .await;
    let v = start(
        4,
        roles(false, true),
        sha_registry(),
        chain.clone(),
        false,
    )
    .await;
    let nodes = [&c1, &c2, &c3, &v];
    connect_all(&nodes).await;
    let miner = spawn_miner(chain.clone());
    upload_input(&c1, &data).await;
    let job = publish_job(&c1, spec).await;
    wait_dump(90, "job finalisé", &nodes, &job, || {
        finalized_everywhere(&nodes, &job)
    })
    .await;

    // Un nœud arrive après la fin : il n'a rien vu. Il apprend le job (annonce du comité),
    // puis rattrape l'historique de tous les auteurs par curseur.
    let late = start(
        7,
        roles(false, false),
        sha_registry(),
        chain.clone(),
        false,
    )
    .await;
    connect_new(&late, &nodes).await;
    // Même journal (mêmes têtes de chaîne) avant de comparer : la phase 2 tranche les claims
    // par ordre total, donc deux nœuds ne sont d'accord qu'une fois les mêmes événements reçus.
    wait_dump(60, "retardataire à jour", &[&late], &job, || {
        finalized_everywhere(&[&late], &job) && same_journal(&late, &c1, &job)
    })
    .await;

    let reference = c1.node.shared().job_state(&job).unwrap();
    let caught_up = late.node.shared().job_state(&job).unwrap();
    for (a, b) in reference.tasks.iter().zip(&caught_up.tasks) {
        assert_eq!(a.finalized, b.finalized, "même état dérivé que les autres");
        assert_eq!(a.attempts.len(), b.attempts.len());
    }
    miner.abort();
    for node in [c1, c2, c3, v, late] {
        node.node.shutdown().await;
    }
}
