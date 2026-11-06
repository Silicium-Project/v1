//! Types de base de V1 : octets de taille fixe, hex strict, erreurs communes.
//!
//! Ce crate n'a aucune logique cryptographique : voir `sil-crypto`.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod bytes;
mod error;
pub mod hex;
mod task;

pub use bytes::{ContentId, Hash32, JobId, PublicKey, Signature};
pub use error::TypeError;
pub use task::TaskRef;

#[cfg(test)]
mod tests;
