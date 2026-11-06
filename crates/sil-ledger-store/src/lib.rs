//! Stockage des événements : fichiers `seg-NNNNNNNN.log`, un enregistrement par événement.
//!
//! Enregistrement : `longueur u32 LE | octets canoniques de l'événement | 4 octets de contrôle`
//! (début de SHA-256 de l'événement). Un enregistrement incomplet ou faux **en fin du dernier
//! segment** est une écriture interrompue : il est tronqué. Partout ailleurs c'est une
//! corruption, signalée sans rien réparer. Voir docs/technical/protocol/storage.md.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod config;
mod error;
mod record;
mod segment;
mod store;

pub use config::{FsyncPolicy, StoreConfig};
pub use error::StoreError;
pub use store::{EventStore, OpenReport};

#[cfg(test)]
mod tests;
