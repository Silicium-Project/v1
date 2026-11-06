//! Binaire `silicium-node` (sans chaîne Solana).
//!
//! Usage : `silicium-node [--config fichier.toml] [--roles coordinator,worker,verifier]`.
//!
//! Sans chaîne, les workers refusent de travailler (la garde d'escrow ne peut rien vérifier) et
//! aucun règlement ne part. Pour régler sur Solana, utiliser le binaire de `chain/node`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    sil_node::cli::main_with(sil_node::cli::without_chain).await
}
