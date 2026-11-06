//! Nœud complet (docs/technical/architecture.md, « Nœud »).
//!
//! Ce crate **assemble** ; la logique métier vit dans les crates qu'il compose. Il lance des
//! acteurs indépendants qui ne communiquent que par l'état partagé ([`shared::Shared`]) et
//! par le réseau :
//!
//! | Acteur | Rôle |
//! |---|---|
//! | `gossip_loop` | reçoit les messages validés, comble les trous, rediffuse les têtes de chaîne |
//! | `coordinator_loop` | membre du comité : vote claims, vérificateurs, finalisation ; signe les règlements |
//! | `worker_loop` | réclame des fragments, les exécute, publie le résultat |
//! | `verifier_loop` | propose de vérifier, recalcule un échantillon, publie le verdict |
//! | `settlement_loop` | fait avancer l'outbox de règlement et la sauvegarde |
//! | API de contrôle | `sil-control-api` branchée sur [`backend::NodeBackend`] |
//!
//! Les clés sont encore dans le même processus (séparation en processus signataire : à faire).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod backend;
mod blob;
pub mod cli;
mod coordinator_loop;
mod error;
mod fetch;
mod gossip_loop;
mod ingest;
mod input;
mod options;
mod presence_loop;
mod providers;
mod reputation_feed;
mod runner;
mod settlement_loop;
mod shared;
mod validator;
mod verifier_loop;
mod wire;
mod work;
mod worker_data;
mod worker_loop;

pub use backend::NodeBackend;
pub use error::NodeError;
pub use options::{ControlOptions, NodeOptions, Roles};
pub use runner::Node;
pub use shared::Shared;
pub use wire::Envelope;

#[cfg(test)]
mod tests;
