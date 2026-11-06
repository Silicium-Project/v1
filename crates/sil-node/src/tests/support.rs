use crate::{ControlOptions, Node, NodeOptions, Roles};
use sil_chain_core::Escrow;
use sil_content::Manifest;
use sil_crypto::{MasterSeed, Purpose};
use sil_ledger::JobSpec;
use sil_net::{PeerId, Protocol};
use sil_settlement::{terms_of, testing::FakeChain};
use sil_types::{Hash32, JobId, PublicKey};
use sil_wl_sha256::Sha256Workload;
use sil_workload_api::Registry;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::sleep;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

pub fn ledger_key(seed: u8) -> PublicKey {
    MasterSeed::from_bytes([seed; 32])
        .derive(Purpose::Ledger)
        .public()
}

pub fn roles(coordinator: bool, worker: bool, verifier: bool) -> Roles {
    Roles {
        coordinator,
        worker,
        verifier,
    }
}

pub fn sha_registry() -> Registry {
    Registry::new().with(Arc::new(Sha256Workload))
}

pub struct TestNode {
    pub node: Node,
    pub dir: tempfile::TempDir,
    pub api: Option<SocketAddr>,
    pub key: PublicKey,
    pub peer: PeerId,
}

pub async fn start(
    seed: u8,
    roles: Roles,
    registry: Registry,
    chain: Arc<FakeChain>,
    with_api: bool,
) -> TestNode {
    let dir = tempfile::tempdir().unwrap();
    start_in(dir, seed, roles, registry, chain, with_api).await
}

pub async fn start_in(
    dir: tempfile::TempDir,
    seed: u8,
    roles: Roles,
    registry: Registry,
    chain: Arc<FakeChain>,
    with_api: bool,
) -> TestNode {
    let mut options = NodeOptions::new(dir.path());
    options.roles = roles;
    options.tick_ms = 100;
    options.trusted_requesters = vec![ledger_key(100)];
    options.listen = vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()];
    if with_api {
        options.control = Some(ControlOptions {
            bind: "127.0.0.1:0".parse().unwrap(),
            token_file: dir.path().join("control.token"),
        });
    }
    let seed_value = MasterSeed::from_bytes([seed; 32]);
    let node = Node::start(options, seed_value, registry, chain)
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

impl TestNode {
    pub fn token(&self) -> String {
        let text = std::fs::read_to_string(self.dir.path().join("control.token")).unwrap();
        let line = text
            .lines()
            .find(|l| l.starts_with("read,submit:"))
            .unwrap();
        line.split_once(':').unwrap().1.to_string()
    }
}

/// Connecte chaque nœud à tous les autres et laisse les abonnements gossip se propager.
pub async fn connect_all(nodes: &[&TestNode]) {
    for (i, from) in nodes.iter().enumerate() {
        for to in nodes.iter().skip(i + 1) {
            let address = loop {
                if let Some(a) = to.node.net().listen_addrs().await.unwrap().first() {
                    break a.clone();
                }
                sleep(Duration::from_millis(50)).await;
            };
            from.node
                .net()
                .dial(address.with(Protocol::P2p(to.peer)))
                .await
                .unwrap();
        }
    }
    wait_for(20, "maillage complet", || true).await;
    for node in nodes {
        let want = nodes.len() - 1;
        let started = Instant::now();
        while node.node.net().peers().await.unwrap().len() < want {
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "pairs manquants"
            );
            sleep(Duration::from_millis(100)).await;
        }
    }
    sleep(Duration::from_millis(2500)).await; // abonnements gossipsub
}

/// Attend qu'une condition soit vraie, sinon fait échouer le test.
pub async fn wait_for(seconds: u64, what: &str, mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(seconds),
            "délai dépassé : {what}"
        );
        sleep(Duration::from_millis(100)).await;
    }
}

/// Résumé lisible de l'état d'un job sur des nœuds (diagnostic d'un délai dépassé).
pub fn dump(nodes: &[&TestNode], job: &JobId) -> String {
    let mut out = String::new();
    for (i, n) in nodes.iter().enumerate() {
        let shared = n.node.shared();
        out += &format!(
            "nœud {i} pairs=? job={:?}
",
            shared.job_state(job).map(|s| {
                s.tasks
                    .iter()
                    .map(|t| {
                        format!(
                            "[{:?} next={:?} attempts={} verifiers={:?}]",
                            t.status,
                            t.next_attempt,
                            t.attempts.len(),
                            t.attempts.last().map(|a| (
                                a.result.is_some(),
                                a.verifiers.len(),
                                a.verifiers.iter().map(|v| v.verdict).collect::<Vec<_>>()
                            ))
                        )
                    })
                    .collect::<Vec<_>>()
            })
        );
    }
    out
}

/// Comme `wait_for`, mais affiche l'état des nœuds si le délai est dépassé.
pub async fn wait_dump(
    seconds: u64,
    what: &str,
    nodes: &[&TestNode],
    job: &JobId,
    mut condition: impl FnMut() -> bool,
) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(seconds),
            "délai dépassé : {what}
{}",
            dump(nodes, job)
        );
        sleep(Duration::from_millis(100)).await;
    }
}

/// Taille de l'entrée de test : deux fragments de 10 000 octets (unités).
pub const INPUT_LEN: u32 = 20_000;

pub fn input_bytes() -> Vec<u8> {
    (0..INPUT_LEN)
        .map(|i| (i.wrapping_mul(31) ^ (i >> 5)) as u8)
        .collect()
}

pub fn input_hash(bytes: &[u8]) -> Hash32 {
    Hash32::from_bytes(*Manifest::of_bytes(bytes).id().as_bytes())
}

pub fn job_spec(committee: &[PublicKey], input: Hash32) -> JobSpec {
    JobSpec {
        version: 1,
        requester: ledger_key(100),
        nonce: now_ms(),
        workload: "calcul/sha256".into(),
        input,
        input_size: u64::from(INPUT_LEN),
        width: 0,
        height: 0,
        fragments: 2,
        committee: committee.to_vec(),
        threshold: 2,
        verifications_required: 1,
        lease_ms: 60_000,
        max_attempts: 3,
        reward_per_unit: 1,
        verifier_reward_per_unit: 1,
        expires_at_ms: now_ms() + 600_000,
    }
}

/// Un escrow financé pour `spec` (son budget : récompenses et frais de chaque fragment).
pub fn escrow_for(spec: &JobSpec) -> Escrow {
    let budget = terms_of(spec).max_payout().unwrap();
    Escrow::open(
        spec.job_id().unwrap(),
        terms_of(spec),
        budget,
        &spec.requester,
    )
    .unwrap()
}

/// Fait atterrir les transactions de la chaîne simulée en continu.
pub fn spawn_miner(chain: Arc<FakeChain>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            chain.mine();
            sleep(Duration::from_millis(100)).await;
        }
    })
}

/// Connecte un nœud tout neuf à des nœuds déjà en place (sans rouvrir leurs connexions).
pub async fn connect_new(new: &TestNode, existing: &[&TestNode]) {
    for to in existing {
        let address = loop {
            if let Some(a) = to.node.net().listen_addrs().await.unwrap().first() {
                break a.clone();
            }
            sleep(Duration::from_millis(50)).await;
        };
        new.node
            .net()
            .dial(address.with(Protocol::P2p(to.peer)))
            .await
            .unwrap();
    }
    let started = Instant::now();
    while new.node.net().peers().await.unwrap().len() < existing.len() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "pairs manquants"
        );
        sleep(Duration::from_millis(100)).await;
    }
    sleep(Duration::from_millis(1500)).await;
}

/// Soumissions de résultats vues par les workers : la preuve qu'un nœud a calculé.
pub fn submits_of(nodes: &[&TestNode]) -> u64 {
    nodes
        .iter()
        .map(|n| {
            crate::worker_data::report(n.node.shared())["events_by_kind"]["submit"]
                .as_u64()
                .unwrap_or(0)
        })
        .sum()
}
