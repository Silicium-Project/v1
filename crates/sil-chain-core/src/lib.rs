//! Règles de l'escrow, sans dépendance à Solana (docs/technical/protocol/settlement.md).
//!
//! C'est le **modèle de référence** du programme on-chain : le programme Anchor ne doit
//! contenir que l'appel de ces fonctions (après avoir extrait les signataires vérifiés par le
//! précompilé Ed25519) et les transferts de jetons. Les règles sont ici parce qu'elles se
//! testent sur n'importe quelle machine, y compris par tirage aléatoire d'invariants.
//!
//! Invariants garantis (testés) :
//! - `payé + remboursé ≤ déposé`, toujours ;
//! - un fragment est réglé **au plus une fois** ;
//! - un règlement exige la signature de **chaque** approbateur, éligible (ADR 0006, 0007) ;
//! - après remboursement, plus aucun règlement ;
//! - le remboursement n'est possible qu'après l'échéance **plus un délai de grâce**, pour
//!   laisser atterrir les règlements en vol ;
//! - aucun calcul ne déborde (arithmétique contrôlée).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;
mod escrow;
mod penalty;
mod refund;
pub mod reputation;
pub mod reserve;
mod settle;
mod terms;
pub mod units;
pub mod wire;

pub use error::RuleError;
pub use escrow::{Escrow, HeldEntry, SettledFragment};
pub use penalty::{Penalty, PenaltyArgs, ToleranceProof};
pub use settle::{Payout, SettleArgs, Settlement};
pub use terms::{EscrowTerms, MAX_APPROVERS, REFUND_GRACE_MS, SETTLEMENT_FEE};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_random;
#[cfg(test)]
mod tests_penalty;
#[cfg(test)]
mod tests_reputation;
#[cfg(test)]
mod wire_vectors;
