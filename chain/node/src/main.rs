//! `silicium-node` avec chaîne Solana : même démarrage que `sil-node`, plus le règlement des jobs
//! sur l'escrow Solana (`[chain]` dans la configuration).
//!
//! Hors du workspace du nœud : les dépendances Solana sont lourdes et ne doivent pas entrer dans
//! les tests du réseau. Voir `chain/README.md`.

use sil_settlement::Chain;
use std::{process::ExitCode, sync::Arc};

fn solana_chain(config: &sil_config::ChainConfig) -> Result<Arc<dyn Chain>, String> {
    let program =
        sil_settlement_solana::parse_program_id(&config.program_id).map_err(|e| e.to_string())?;
    let payer =
        sil_settlement_solana::read_payer(&config.payer_keypair).map_err(|e| e.to_string())?;
    tracing::info!(rpc = %config.rpc_url, program = %program, "chaîne Solana configurée");
    Ok(Arc::new(sil_settlement_solana::SolanaChain::new(
        &config.rpc_url,
        program,
        payer,
    )))
}

#[tokio::main]
async fn main() -> ExitCode {
    sil_node::cli::main_with(solana_chain).await
}
