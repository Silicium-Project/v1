use super::{client::*, liar::liar_registry, scenario::*, support::*};
use sil_ledger::{JobSpec, TaskStatus};
use sil_settlement::{
    ChainError, Status,
    testing::{FakeChain, Op},
};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_runs_end_to_end_over_real_nodes_despite_chain_outages() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let spec = job_spec(input_hash(&data));
    chain.insert_escrow(escrow_for(&spec));
    // La chaîne est capricieuse : lectures limitées et envois qui échouent.
    chain.fail(Op::Escrow, ChainError::Transient("429".into()), 4);
    chain.fail(Op::Submit, ChainError::Transient("délai".into()), 3);

    let c1 = start(1, roles(true, false), sha_registry(), chain.clone(), true).await;
    let c2 = start(2, roles(true, false), sha_registry(), chain.clone(), false).await;
    let c3 = start(3, roles(true, false), sha_registry(), chain.clone(), false).await;
    let v = start(4, roles(false, true), sha_registry(), chain.clone(), false).await;
    let nodes = [&c1, &c2, &c3, &v];
    connect_all(&nodes).await;
    let miner = spawn_miner(chain.clone());

    upload_input(&c1, &data).await;
    let job = publish_job(&c1, spec).await;

    wait_for(90, "job finalisé partout", || {
        finalized_everywhere(&nodes, &job)
    })
    .await;
    let submits = submits_of(&nodes);
    assert!(submits >= 1, "no node reports a submit: {submits}");
    wait_for(60, "règlements confirmés", || {
        chain
            .escrow_now(&job)
            .is_some_and(|e| e.paid() == 2 * (20_000 + sil_chain_core::SETTLEMENT_FEE))
    })
    .await;

    for fragment in 0..2u32 {
        let (claimant, result) = stored_result(&nodes, &job, fragment);
        assert_eq!(
            result,
            expected_digest(&data, fragment as usize),
            "fragment {fragment}"
        );
        assert!(
            nodes.iter().any(|node| node.key == claimant),
            "un worker du réseau a calculé"
        );
        let settled = chain.escrow_now(&job).unwrap();
        assert_eq!(
            settled.settled(fragment).unwrap().claimant,
            claimant,
            "le bon worker est payé"
        );
    }
    // Les trois coordinateurs ont soumis le même règlement : tous finissent confirmés,
    // et la chaîne n'a payé qu'une fois (vérifié plus haut).
    wait_for(30, "outbox confirmées chez les coordinateurs", || {
        [&c1, &c2, &c3].iter().all(|n| {
            let report = n.node.shared().settlement_report();
            report.len() == 2 && report.iter().all(|(_, s)| *s == Status::Confirmed)
        })
    })
    .await;
    miner.abort();
    for node in [c1, c2, c3, v] {
        node.node.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_lying_worker_is_rejected_and_an_honest_one_is_paid_instead() {
    let chain = Arc::new(FakeChain::new());
    // L'honnête H ne voit l'escrow (donc ne réclame rien) qu'une fois le menteur écarté.
    let chain_h = Arc::new(FakeChain::new());
    let data = input_bytes();
    let spec = job_spec(input_hash(&data));
    chain.insert_escrow(escrow_for(&spec));

    let c1 = start(1, roles(false, false), sha_registry(), chain.clone(), true).await;
    let c2 = start(2, roles(false, false), sha_registry(), chain.clone(), false).await;
    let c3 = start(3, roles(false, false), sha_registry(), chain.clone(), false).await;
    let liar = start(5, roles(true, false), liar_registry(), chain.clone(), false).await;
    let honest = start(
        6,
        roles(true, false),
        sha_registry(),
        chain_h.clone(),
        false,
    )
    .await;
    let v = start(4, roles(false, true), sha_registry(), chain.clone(), false).await;
    let nodes = [&c1, &c2, &c3, &liar, &honest, &v];
    connect_all(&nodes).await;
    let miner = spawn_miner(chain.clone());

    upload_input(&c1, &data).await;
    let job = publish_job(&c1, spec.clone()).await;

    wait_dump(
        60,
        "le menteur est rejeté sur les deux fragments",
        &nodes,
        &job,
        || {
            c1.node.shared().job_state(&job).is_some_and(|s| {
                s.tasks.iter().all(|t| {
                    t.attempts.first().is_some_and(|a| a.claimant == liar.key)
                        && matches!(t.next_attempt, Some(2))
                })
            })
        },
    )
    .await;
    // Le menteur est écarté : l'honnête voit maintenant l'escrow et peut réclamer.
    chain_h.insert_escrow(escrow_for(&spec));

    wait_dump(120, "job finalisé", &nodes, &job, || {
        finalized_everywhere(&nodes, &job)
    })
    .await;
    wait_for(60, "règlements confirmés", || {
        chain
            .escrow_now(&job)
            .is_some_and(|e| e.paid() == 2 * (20_000 + sil_chain_core::SETTLEMENT_FEE))
    })
    .await;

    let state = c1.node.shared().job_state(&job).unwrap();
    for (fragment, task) in state.tasks.iter().enumerate() {
        assert_eq!(task.status, TaskStatus::Finalized);
        assert_eq!(
            task.attempts[0].claimant, liar.key,
            "le menteur a essayé en premier"
        );
        assert!(task.attempts.len() >= 2);
        let finalized = task.finalized.as_ref().unwrap();
        assert_eq!(finalized.claimant, honest.key, "l'honnête est retenu");
        let (_, result) = stored_result(&nodes, &job, fragment as u32);
        assert_eq!(result, expected_digest(&data, fragment));
        let paid_to = chain
            .escrow_now(&job)
            .unwrap()
            .settled(fragment as u32)
            .unwrap()
            .claimant;
        assert_eq!(paid_to, honest.key, "le menteur n'est jamais payé");
    }
    // Les coordinateurs ont retenu l'issue : l'honnête monte, le menteur descend.
    wait_for(20, "réputation mise à jour", || {
        let reputation = crate::shared::lock(&c1.node.shared().reputation);
        let now = c1.node.shared().now_ms();
        reputation.score(&honest.key, now) > 0.6 && reputation.score(&liar.key, now) < 0.4
    })
    .await;
    miner.abort();
    for node in [c1, c2, c3, liar, honest, v] {
        node.node.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_job_survives_the_loss_of_a_worker_node() {
    let chain = Arc::new(FakeChain::new());
    let data = input_bytes();
    let spec: JobSpec = job_spec(input_hash(&data));
    chain.insert_escrow(escrow_for(&spec));

    let c1 = start(1, roles(true, false), sha_registry(), chain.clone(), true).await;
    let c2 = start(2, roles(true, false), sha_registry(), chain.clone(), false).await;
    let c3 = start(3, roles(true, false), sha_registry(), chain.clone(), false).await;
    let v = start(4, roles(false, true), sha_registry(), chain.clone(), false).await;
    connect_all(&[&c1, &c2, &c3, &v]).await;
    // Un membre sur trois tombe avant le job : 2 sur 3 suffisent (majorité stricte).
    c3.node.shutdown().await;
    let alive = [&c1, &c2, &v];
    let miner = spawn_miner(chain.clone());

    upload_input(&c1, &data).await;
    let job = publish_job(&c1, spec).await;

    let started = std::time::Instant::now();
    while !finalized_everywhere(&alive, &job) {
        assert!(
            started.elapsed().as_secs() < 90,
            "délai : job non finalisé
{}",
            dump(&alive, &job)
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    wait_for(60, "payé", || {
        chain
            .escrow_now(&job)
            .is_some_and(|e| e.paid() == 2 * (20_000 + sil_chain_core::SETTLEMENT_FEE))
    })
    .await;
    miner.abort();
    for node in [c1, c2, v] {
        node.node.shutdown().await;
    }
}
