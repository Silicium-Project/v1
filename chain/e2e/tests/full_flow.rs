//! Flux complet sur validateur local : un job `calcul/sha256` et un job `rendu/blocs`, de la
//! publication au règlement on-chain, par trois membres du comité et un vérificateur.
//! Voir chain/e2e/README.md.

mod support;

use sil_node::Roles;
use sil_settlement::Chain;
use sil_settlement_solana::SolanaChain;
use sil_wl_render::RenderWorkload;
use sil_wl_sha256::Sha256Workload;
use sil_workload_api::Registry;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::sync::Arc;
use support::{
    World,
    chain::{SOL, address, airdrop, config, ledger_identity, ledger_keypair},
    job::{Case, run},
    nodes::{Running, connect_all, start},
    render::engine,
};

const REQUESTER: u8 = 100;
const COMMITTEE: [u8; 3] = [21, 22, 23];
const VERIFIER: u8 = 24;

fn registry() -> Registry {
    Registry::new()
        .with(Arc::new(Sha256Workload))
        .with(Arc::new(RenderWorkload::new(engine())))
}

/// En-tête (largeur, hauteur en u32 little-endian) puis une scène de 100 octets.
fn render_input(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend((0..100u32).map(|i| (i * 7) as u8));
    bytes
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs solana-test-validator, see chain/e2e/README.md"]
async fn a_sha256_job_and_a_render_job_settle_on_chain_with_the_right_payouts() {
    // Journaux des nœuds : `SIL_LOG` (directives tracing), `warn` par défaut.
    let level = std::env::var("SIL_LOG").unwrap_or_else(|_| "warn".into());
    let _ = sil_telemetry::init(&sil_telemetry::TelemetryConfig { level, json: false });
    let (url, program) = config();
    let requester = ledger_identity(REQUESTER);
    let requester_key = ledger_keypair(REQUESTER);
    let requester_chain = SolanaChain::new(&url, program, ledger_keypair(REQUESTER));
    airdrop(&requester_chain, &requester_key.pubkey(), 5 * SOL);

    // Chaque nœud : une clé de paiement séparée (les frais), et son adresse de ledger (les gains).
    let mut nodes: Vec<Running> = Vec::new();
    for seed in COMMITTEE.iter().copied().chain([VERIFIER]) {
        let roles = if seed == VERIFIER {
            Roles {
                coordinator: false,
                worker: false,
                verifier: true,
            }
        } else {
            Roles {
                coordinator: true,
                worker: true,
                verifier: false,
            }
        };
        let payer = Keypair::new();
        airdrop(&requester_chain, &payer.pubkey(), SOL);
        airdrop(
            &requester_chain,
            &address(&ledger_identity(seed).public()),
            SOL,
        );
        let chain: Arc<dyn Chain> = Arc::new(SolanaChain::new(&url, program, payer));
        nodes.push(start(seed, roles, registry(), chain, requester.public()).await);
    }
    let refs: Vec<&Running> = nodes.iter().collect();
    connect_all(&refs).await;

    let world = World {
        program,
        requester,
        requester_key,
        requester_chain,
        nodes,
        committee: COMMITTEE
            .iter()
            .map(|s| ledger_identity(*s).public())
            .collect(),
    };
    let sha = Case {
        workload: "calcul/sha256",
        input: (0..20_000u32)
            .map(|i| (i.wrapping_mul(31) ^ (i >> 5)) as u8)
            .collect(),
        width: 0,
        height: 0,
        fragments: 2,
    };
    run(&world, &sha, 1).await;

    let render = Case {
        workload: "rendu/blocs",
        input: render_input(64, 40),
        width: 64,
        height: 40,
        fragments: 3,
    };
    run(&world, &render, 2).await;

    for node in world.nodes {
        node.node.shutdown().await;
    }
}
