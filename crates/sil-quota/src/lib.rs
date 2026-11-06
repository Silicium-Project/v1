//! Bornes de ressources. Corrige le défaut de V0 : aucune limite globale de journal.
//!
//! - [`LedgerLimits`] : plafonds du journal (jobs, événements, octets, tampon).
//! - [`TokenBucket`] : limiteur de débit, horloge injectée (déterministe).
//! - [`KeyedLimiter`] : un seau par clé (pair, auteur) avec nombre de clés **borné**.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod bucket;
mod keyed;
mod limits;

pub use bucket::TokenBucket;
pub use keyed::KeyedLimiter;
pub use limits::LedgerLimits;

#[cfg(test)]
mod tests;
