//! Règlement on-chain des fragments finalisés (docs/technical/protocol/settlement.md).
//!
//! Pipeline : un fragment `Finalized` dans le journal → [`SettlementClaim`] → signature par
//! `threshold` membres du comité ([`Authorization`]) → [`Outbox`] → [`Chain::submit`].
//!
//! Garanties de l'outbox (testées contre une chaîne qui tombe en panne) :
//! - **idempotente** : un même règlement a un identifiant unique, jamais deux entrées ;
//! - **relecture avant tout (re)envoi** : l'état de l'escrow est lu *avant* de soumettre, donc
//!   une transaction qui a atterri malgré une erreur réseau n'est jamais renvoyée en double ;
//! - **reprenable** : l'état est sauvegardé atomiquement, un redémarrage ne perd ni ne
//!   duplique rien ;
//! - **états honnêtes** : `Pending`, `Submitted`, `Unknown` (chaîne injoignable), `Blocked`
//!   (pas de fonds pour les frais), `Confirmed`, `Rejected` : un règlement n'est « confirmé »
//!   que lu sur la chaîne.
//!
//! Aucun client Solana ici : un adaptateur implémente [`Chain`] (voir le document).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod advance;
mod backoff;
mod chain;
mod claim;
mod error;
mod guard;
mod outbox;
mod store;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use backoff::OutboxConfig;
pub use chain::{Chain, ChainError, TxId, TxStatus};
pub use claim::{Authorization, IntentId, SettlementClaim};
pub use error::SettlementError;
pub use guard::{GuardError, check_escrow, terms_of};
pub use outbox::{Outbox, Phase, Status};
pub use store::OutboxFile;

#[cfg(test)]
mod tests;
