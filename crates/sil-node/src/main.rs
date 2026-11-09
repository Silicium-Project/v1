//! Binaire `silicium-node` (sans chaîne Solana).
//!
//! Usage : `silicium-node [--config fichier.toml] [--roles worker,verifier]`.
//! `--public-key` affiche la clé publique du nœud (identité du nœud) puis s'arrête.
//!
//! Sans chaîne, les workers refusent de travailler (la garde d'escrow ne peut rien vérifier) et
//! aucun règlement ne part. Pour régler sur Solana, utiliser le binaire de `chain/node`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    sil_node::cli::main_with(sil_node::cli::without_chain).await
}
