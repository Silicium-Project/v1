//! Programme d'escrow de Silicium (V1). Une instruction = une règle de `sil-chain-core`.
//!
//! - `process` : dispatch, ouverture, dépôt, remboursement.
//! - `settle` : règlement d'un fragment, signatures des approbateurs lues dans les instructions
//!   Ed25519 précompilées de la même transaction.
//! - `state` : chargement et sauvegarde du compte d'escrow (CBOR canonique, longueur en tête).

pub mod penalty;
pub mod process;
pub mod settle;
pub mod state;
pub mod verifier;

#[cfg(not(feature = "no-entrypoint"))]
use process::process_instruction;
#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);
