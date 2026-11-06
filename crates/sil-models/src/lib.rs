//! Catalogue de modèles. Un modèle = des fichiers, chacun épinglé par `ContentId` et taille.
//!
//! La distribution réutilise `sil-content` (morceaux vérifiés, reprise, quotas). Ce crate
//! ajoute la **politique** : un nœud ne télécharge ni ne sert que des identifiants du
//! catalogue (V0 : le hash épinglé était revérifié seulement après le téléchargement complet).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod catalog;
mod error;
mod validate;

pub use catalog::{Catalog, Model, ModelFile};
pub use error::ModelError;

#[cfg(test)]
mod tests;
