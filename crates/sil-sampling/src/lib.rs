//! Échantillonnage de vérification (docs/technical/protocol/verification.md).
//!
//! **Graine** : le vérificateur signe `(tâche, tentative, résultat)` dans le domaine
//! `Sampling`. Ed25519 est déterministe : il ne peut pas « tirer » une signature favorable,
//! et n'importe qui peut revérifier la graine. Le worker ne peut pas l'anticiper (il faudrait
//! la clé du vérificateur), et personne ne la choisit (le vérificateur et le résultat la déterminent, pas la
//! graine de chacun). Avec V vérificateurs, chacun tire son propre échantillon : pour tricher
//! sans être vu il faut contrôler **tous** les vérificateurs.
//!
//! **Combien** : [`samples_needed`] donne la taille d'échantillon pour détecter, avec une
//! probabilité cible, un tricheur qui falsifie au moins une fraction donnée des blocs
//! (V0 : un cinquième des blocs, 8 au plus, soit 8 % de détection pour un bloc falsifié sur 100).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod draw;
mod math;
mod seed;

pub use draw::sample_indices;
pub use math::{detection_probability, samples_needed};
pub use seed::{SamplingSeed, SeedError, challenge_message};

#[cfg(test)]
mod tests;
