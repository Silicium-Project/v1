//! Workload `calcul/sha256` : chaque fragment est une plage d'octets de l'entrée dont on
//! publie le SHA-256. Politique `Recompute` : un vérificateur refait le hash de la plage.
//! Le résultat du job est le SHA-256 de la concaténation ordonnée des hash de fragments.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod range;
mod workload;

pub use workload::Sha256Workload;

#[cfg(test)]
mod tests;
