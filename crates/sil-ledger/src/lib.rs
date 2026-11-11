//! Journal de V1. Voir docs/technical/protocol/ledger.md.
//!
//! - Un **job** commence par un [`SignedJobSpec`] (requester).
//! - Chaque auteur a une **chaîne par job** (séquence + hash précédent) : pas d'intrication
//!   entre jobs, rattrapage et quotas par job.
//! - Les **décisions** (accepter un claim, un vérificateur, finaliser) sont des votes de
//!   vérificateurs ; le règlement est signé par chacun d'eux (ADR 0007).
//! - `Finalize` embarque un [`sil_attest::Certificate`] : sans vérificateurs indépendants
//!   ayant approuvé, aucune finalisation (ADR 0002).
//! - L'état est une **fonction pure** de l'ensemble d'événements et de l'heure passée en
//!   argument : tous les pairs qui ont les mêmes événements dérivent le même état.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod admission;
mod body;
mod chain;
mod error;
mod event;
mod job;
mod ledger;
mod receipt;
mod spec;
mod state;
mod tolerance;
pub use tolerance::{rules_of as tolerance_rules, runtime_hash};
mod wallet_text;

pub use admission::{Admission, AllowAll};
pub use body::{EventBody, MAX_ATTEMPTS};
pub use error::LedgerError;
pub use event::{Event, Header, NO_PREVIOUS};
pub use ledger::Ledger;
pub use receipt::{Gap, Receipt};
pub use spec::{JobSpec, SignedJobSpec};
pub use state::{
    AttemptState, Finalized, JobState, JobStatus, OpenOffer, ResultState, TaskState, TaskStatus,
    VerifierState,
};

#[cfg(test)]
mod tests;
