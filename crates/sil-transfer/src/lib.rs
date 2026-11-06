//! Transfert de contenu entre pairs.
//!
//! - [`StoreProvider`] expose un `ContentStore` au réseau (manifeste, morceaux).
//! - [`fetch`] télécharge un contenu depuis une liste de pairs : manifeste vérifié contre
//!   l'identifiant, chaque morceau vérifié avant écriture, reprise auprès du pair suivant
//!   si l'un échoue ou ment (les morceaux déjà valides sont conservés).
//!
//! Politique : `fetch` ne décide pas *quoi* télécharger. L'appelant vérifie avant (par
//! exemple `Catalog::is_pinned` pour un modèle, l'`input` d'un `JobSpec` pour un job).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;
mod fetch;
mod provider;

pub use error::TransferError;
pub use fetch::{TransferConfig, fetch};
pub use provider::StoreProvider;

#[cfg(test)]
mod tests;
