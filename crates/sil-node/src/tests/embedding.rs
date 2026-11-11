//! Job d'embedding de bout en bout sur deux nœuds réels : un worker calcule les vecteurs, un
//! vérificateur les recalcule par tolérance, le fragment est finalisé. Ignoré par défaut : il
//! demande ONNX Runtime, le modèle et le tokenizer (voir crates/sil-wl-embed/tests/reference.rs).

use super::{client::publish_job, scenario::*, support::*};
use sil_wl_embed::{EmbedWorkload, Engine};
use sil_workload_api::{Registry, Workload};
use std::{path::PathBuf, sync::Arc};

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("variable {name} absente")))
}

fn passages() -> Vec<u8> {
    [
        "Le calcul distribué permet de répartir un travail entre plusieurs machines.",
        "Distributed computing spreads a workload across many machines.",
        "Verteiltes Rechnen verteilt eine Aufgabe auf viele Maschinen.",
        "Le chat dort sur le canapé près de la fenêtre.",
        "The cat sleeps on the sofa near the window.",
        "Facture n° 2026-0412 : montant TTC 1 248,50 euros.",
        "SELECT id, label FROM jobs WHERE state = 'finalized';",
        "Le réseau paie les machines qui vérifient un résultat honnête.",
    ]
    .join("\n")
    .into_bytes()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "demande ONNX Runtime, le modèle et le tokenizer (variables SIL_EMBED_*)"]
async fn an_embedding_job_is_finalized_by_tolerance_over_real_nodes() {
    let engine = Engine::open(
        &env_path("SIL_EMBED_RUNTIME"),
        &env_path("SIL_EMBED_MODEL"),
        &env_path("SIL_EMBED_TOKENIZER"),
    )
    .unwrap();
    // Un seul moteur partagé par les deux nœuds du test (même processus).
    let workload: Arc<dyn Workload> = Arc::new(EmbedWorkload::new(engine));
    let registry = || Registry::new().with(workload.clone());

    let data = passages();
    let mut spec = job_spec(input_hash(&data));
    spec.workload = "embedding/e5-small".into();
    spec.input_size = data.len() as u64;

    let chain = Arc::new(sil_settlement::testing::FakeChain::new());
    chain.insert_escrow(escrow_for(&spec));

    let worker = start(1, roles(true, false), registry(), chain.clone(), true).await;
    let verifier = start(2, roles(false, true), registry(), chain.clone(), false).await;
    let nodes = [&worker, &verifier];
    connect_all(&nodes).await;
    let miner = spawn_miner(chain.clone());

    upload_input(&worker, &data).await;
    let job = publish_job(&worker, spec).await;

    wait_for(300, "job d'embedding finalisé partout", || {
        finalized_everywhere(&nodes, &job)
    })
    .await;

    wait_for(60, "règlement confirmé on-chain", || {
        chain
            .escrow_now(&job)
            .is_some_and(|escrow| escrow.paid() > 0)
    })
    .await;
    miner.abort();
    for node in [worker, verifier] {
        node.node.shutdown().await;
    }
}
