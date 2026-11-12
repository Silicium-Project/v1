use super::{client::publish_job, scenario::*, slow::slow_registry, support::*};
use sil_ledger::{JobSpec, JobStatus};
use sil_settlement::testing::FakeChain;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// Trois calculateurs et un vérificateur, un job de six fragments qui prennent chacun du temps :
/// un calculateur ne doit jamais tenir deux fragments en même temps tant que les autres attendent.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workers_take_one_fragment_at_a_time_while_others_wait() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let base = job_spec(input_hash(&data));
    let spec = JobSpec {
        fragments: 6,
        ..base
    };
    chain.insert_escrow(escrow_for(&spec));
    let registry = || slow_registry(Duration::from_millis(1_500));
    let c1 = start(1, roles(true, false), registry(), chain.clone(), true).await;
    let c2 = start(2, roles(true, false), registry(), chain.clone(), false).await;
    let c3 = start(3, roles(true, false), registry(), chain.clone(), false).await;
    let v = start(4, roles(false, true), sha_registry(), chain.clone(), false).await;
    let nodes = [&c1, &c2, &c3, &v];
    connect_all(&nodes).await;
    let miner = spawn_miner(chain.clone());

    upload_input(&c1, &data).await;
    let job = publish_job(&c1, spec).await;
    wait_for(120, "job finalisé partout", || {
        finalized_everywhere(&nodes, &job)
    })
    .await;

    // Intervalles de calcul par calculateur : (début du claim, soumission).
    let state = c1.node.shared().job_state(&job).expect("job connu");
    assert_eq!(state.status, JobStatus::Finalized);
    let mut by_worker: BTreeMap<_, Vec<(u64, u64)>> = BTreeMap::new();
    for task in &state.tasks {
        for attempt in &task.attempts {
            if let Some(result) = &attempt.result {
                by_worker
                    .entry(attempt.claimant)
                    .or_default()
                    .push((attempt.accepted_at_ms, result.submitted_at_ms));
            }
        }
    }
    for (worker, spans) in &by_worker {
        let overlaps = spans
            .iter()
            .enumerate()
            .flat_map(|(i, a)| spans.iter().skip(i + 1).map(move |b| (a, b)))
            .filter(|(a, b)| a.0 < b.1 && b.0 < a.1)
            .count();
        assert_eq!(
            overlaps, 0,
            "{worker:?} tient plusieurs fragments à la fois : {spans:?}"
        );
    }
    miner.abort();
    for node in [c1, c2, c3, v] {
        node.node.shutdown().await;
    }
}
