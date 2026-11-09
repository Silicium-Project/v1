//! Réputation locale d'un nœud envers les workers (ADR 0003).
//!
//! Corrige V0, où `author_records` comptait les tâches de **n'importe quel** publisher : un
//! attaquant publiait ses propres tâches, se les attribuait, se « vérifiait » et gonflait son
//! score. Ici :
//! - seules comptent les observations d'un **scope de confiance** (un requester que ce nœud
//!   a choisi de croire) ;
//! - les observations où le worker est **le requester ou un insider** sont ignorées
//!   (auto-évaluation) ;
//! - le poids d'un succès croît avec la **valeur** payée, mais est plafonné ;
//! - tout **décroît** avec une demi-vie : une réputation se maintient, ne se capitalise pas ;
//! - un worker sans historique est en **période probatoire** (une tâche à la fois) ;
//! - la table est **bornée**.
//!
//! Limite : sans coût d'entrée (stake), une clé jetée puis recréée retrouve la probation, pas
//! pire. Le dégât d'un Sybil est borné par la probation, pas supprimé.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod config;
mod derived;
mod record;
mod reputation;

pub use config::ReputationConfig;
pub use derived::{Observation, from_observations};
pub use reputation::{Outcome, Reputation, verdict_outcome};

#[cfg(test)]
mod tests;
