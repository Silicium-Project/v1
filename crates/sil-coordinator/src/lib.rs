//! Automate d'un membre du comité (docs/technical/protocol/coordination.md).
//!
//! `Coordinator::decide` lit le journal et rend les votes que **ce membre** doit émettre
//! maintenant. Il ne signe ni n'envoie rien : le nœud ajoute chaque vote au journal et le
//! relaie. Les membres appliquent le même classement sur les mêmes offres, donc convergent
//! sur les mêmes choix ; la décision existe quand `threshold` d'entre eux l'ont émise.
//!
//! Garanties :
//! - **idempotent** : un vote déjà émis n'est jamais réémis (le premier vote d'un membre sur un
//!   emplacement est le seul qui compte dans le journal, voir ledger.md) ;
//! - **jamais de finalisation sans preuve** : `Finalize` n'est proposé que si le journal contient
//!   assez d'approbations de vérificateurs retenus ;
//! - choix des claims et des vérificateurs par **réputation** décroissante, puis ancienneté,
//!   puis identifiant (ordre total, donc déterministe).
//!
//! Non couvert : rotation du comité et coordinateur de secours (ADR 0001) ; la vivacité exige
//! `threshold` membres joignables.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod coordinator;
mod mine;
mod ranking;

pub use coordinator::{Coordinator, leader_index};

#[cfg(test)]
mod tests;
