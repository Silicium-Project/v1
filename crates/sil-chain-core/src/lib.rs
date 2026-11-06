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
//! - un règlement exige `threshold` membres **distincts** du comité ;
//! - après remboursement, plus aucun règlement ;
//! - le remboursement n'est possible qu'après l'échéance **plus un délai de grâce**, pour
//!   laisser atterrir les règlements en vol ;
//! - aucun calcul ne déborde (arithmétique contrôlée).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;
mod escrow;
mod refund;
mod settle;
mod terms;
pub mod units;
pub mod wire;

pub use error::RuleError;
pub use escrow::{Escrow, SettledFragment};
pub use settle::{Payout, SettleArgs};
pub use terms::{EscrowTerms, MAX_APPROVERS, REFUND_GRACE_MS, SETTLEMENT_FEE};

#[cfg(test)]
mod tests;
