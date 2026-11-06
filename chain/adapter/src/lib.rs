//! Adaptateur Solana de `sil-settlement`. Voir docs/technical/protocol/settlement.md.
//!
//! - `tx` : instructions (escrow, règlement avec précompilés Ed25519), adresse du PDA.
//! - `client` : `SolanaChain`, qui implémente `Chain` (lecture de l'escrow, soumission, suivi).
//! - `errors` : traduction des erreurs RPC et de programme vers `ChainError` / `TxStatus`.

pub mod client;
pub mod errors;
pub mod tx;

pub use client::{FEE_RESERVE, SolanaChain, parse_program_id, read_payer};
pub use tx::escrow_address;
