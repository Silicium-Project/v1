//! Un job de bout en bout : entrée, spec signée, escrow ouvert, publication, règlement on-chain,
//! puis contrôle des montants : chaque nœud reçoit exactement ce que ses unités valent.

use super::{
    World,
    chain::{address, balance, confirm, now_ms},
    nodes::{diagnose, publish, upload},
};
use sil_chain_core::{Escrow, reserve};
use sil_chain_core::units::{Shape, fragment_units};
use sil_content::Manifest;
use sil_ledger::JobSpec;
use sil_settlement::{Chain, terms_of};
use sil_settlement_solana::escrow_address;
use sil_types::{Hash32, JobId, PublicKey};
use std::{collections::BTreeMap, time::Duration};

/// Tarif par unité versé au worker qui réalise un fragment.
pub const REWARD_PER_UNIT: u64 = 100;
/// Tarif par unité versé à chaque vérificateur.
pub const VERIFIER_PER_UNIT: u64 = 10;

/// Un job à lancer : workload, entrée telle que stockée, forme de l'image, fragments.
pub struct Case {
    pub workload: &'static str,
    pub input: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub fragments: u32,
}

/// Lance le job, attend son règlement complet et contrôle les montants. Retourne son identifiant.
pub async fn run(world: &World, case: &Case, nonce: u64) -> JobId {
    upload(&world.nodes[0], &case.input).await;
    let spec = JobSpec {
        version: 2,
        requester: world.requester.public(),
        nonce,
        workload: case.workload.into(),
        input: Hash32::from_bytes(*Manifest::of_bytes(&case.input).id().as_bytes()),
        input_size: case.input.len() as u64,
        width: case.width,
        height: case.height,
        fragments: case.fragments,
        verifications_required: 1,
        lease_ms: 60_000,
        max_attempts: 3,
        reward_per_unit: REWARD_PER_UNIT,
        verifier_reward_per_unit: VERIFIER_PER_UNIT,
        expires_at_ms: now_ms() + 3_600_000,
    };
    let terms = terms_of(&spec);
    let deposit = terms.max_payout().expect("budget");
    let job = spec.job_id().expect("identifiant du job");

    // Le requérant dépose le budget dans l'escrow avant toute publication.
    let tx = world
        .requester_chain
        .open_escrow(&world.requester_key, job, terms, deposit)
        .expect("ouverture de l'escrow");
    confirm(&world.requester_chain, &tx);
    let account = escrow_address(&world.program, &job);
    let account_before = balance(&world.requester_chain, &account);
    let before: Vec<u64> = world
        .nodes
        .iter()
        .map(|node| balance(&world.requester_chain, &address(&node.key)))
        .collect();

    // Publication : le réseau calcule, vérifie et règle.
    let published = publish(&world.nodes[0], spec, &world.requester).await;
    assert_eq!(published, job, "identifiant publié");
    wait_settled(world, &job, case.fragments).await;

    // Contrôle : tout le dépôt est réglé, l'escrow ne perd que ce qu'il paie, et chaque nœud
    // gagne exactement ses unités × tarifs.
    let escrow = world
        .requester_chain
        .escrow(&job)
        .expect("lecture de l'escrow")
        .expect("escrow ouvert");
    assert_eq!(escrow.paid(), deposit, "tout le dépôt est réglé");
    let shape = Shape {
        input_size: case.input.len() as u64,
        width: case.width,
        height: case.height,
    };
    let expected = expected_payouts(&escrow, case, shape);
    for (index, node) in world.nodes.iter().enumerate() {
        let after = balance(&world.requester_chain, &address(&node.key));
        let gained = after.saturating_sub(before[index]);
        let want = expected.get(&node.key).copied().unwrap_or(0);
        assert_eq!(gained, want, "gain du nœud {index} (job {})", case.workload);
    }
    let account_after = balance(&world.requester_chain, &account);
    assert_eq!(
        account_before - account_after,
        escrow.paid(),
        "l'escrow ne perd que ce qu'il paie"
    );
    job
}

/// Montant dû à chaque nœud : son tarif fois les unités des fragments qu'il a réalisés ou
/// vérifiés, lus dans l'état réglé de l'escrow.
fn expected_payouts(escrow: &Escrow, case: &Case, shape: Shape) -> BTreeMap<PublicKey, u64> {
    let mut expected = BTreeMap::new();
    for fragment in 0..case.fragments {
        let settled = escrow.settled(fragment).expect("fragment réglé");
        let units = fragment_units(case.workload, shape, case.fragments, fragment).expect("unités");
        *expected.entry(settled.claimant).or_insert(0) += units * REWARD_PER_UNIT;
        // Le vérificateur reçoit la moitié de sa récompense ; l'autre moitié reste en réserve (ADR 0010).
        let (paid, _held) = reserve::split(units * VERIFIER_PER_UNIT);
        for approver in &settled.approvers {
            *expected.entry(*approver).or_insert(0) += paid;
        }
    }
    expected
}

/// Attend que tous les fragments soient réglés on-chain.
async fn wait_settled(world: &World, job: &JobId, fragments: u32) {
    let started = std::time::Instant::now();
    loop {
        let settled = world
            .requester_chain
            .escrow(job)
            .expect("lecture de l'escrow")
            .map_or(0, |escrow| escrow.settled_count());
        if settled == fragments as usize {
            return;
        }
        if started.elapsed() >= Duration::from_secs(240) {
            let mut state = String::new();
            for (index, node) in world.nodes.iter().enumerate() {
                state += &format!("nœud {index}:\n{}", diagnose(node, job).await);
            }
            panic!("règlement incomplet : {settled}/{fragments} fragments\n{state}");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
