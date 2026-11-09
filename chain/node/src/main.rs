//! `silicium-node` avec chaîne Solana : même démarrage que `sil-node`, plus le règlement des jobs
//! sur l'escrow Solana (`[chain]` dans la configuration).
//!
//! Deux modes : `payer_keypair` (ce nœud paie ses règlements et peut relayer ceux des autres) ou
//! `relay_url` (ce nœud lit la chaîne et envoie ses règlements à un relais, sans clé de paiement).
//!
//! Hors du workspace du nœud : les dépendances Solana sont lourdes et ne doivent pas entrer dans
//! les tests du réseau. Voir `chain/README.md`.

mod relay;

use relay::RelayChain;
use sil_settlement::Chain;
use sil_settlement_solana::SolanaChain;
use std::{process::ExitCode, sync::Arc};

fn solana_chain(config: &sil_config::ChainConfig) -> Result<Arc<dyn Chain>, String> {
    let program =
        sil_settlement_solana::parse_program_id(&config.program_id).map_err(|e| e.to_string())?;
    tracing::info!(rpc = %config.rpc_url, program = %program, "chaîne Solana configurée");
    match (&config.payer_keypair, &config.relay_url) {
        (Some(path), _) => {
            let payer = sil_settlement_solana::read_payer(path).map_err(|e| e.to_string())?;
            Ok(Arc::new(SolanaChain::new(&config.rpc_url, program, payer)))
        }
        (None, Some(relay)) => {
            tracing::info!(relay = %relay, "règlement par relais : ce nœud ne paie pas les frais");
            let reader = SolanaChain::read_only(&config.rpc_url, program);
            Ok(Arc::new(RelayChain::new(reader, relay)))
        }
        (None, None) => Err("[chain] : payer_keypair ou relay_url requis".into()),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    sil_node::cli::main_with(solana_chain).await
}
