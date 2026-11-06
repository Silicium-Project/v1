//! Cryptographie de V1.
//!
//! - [`Domain`] : chaque signature est liée à un usage (pas de réutilisation entre types).
//! - [`MasterSeed`] / [`Purpose`] : une graine unique, des clés indépendantes par usage.
//! - [`keyfile`] : stockage de la graine, créé atomiquement en 0600.
//! - [`ct_eq`] : comparaison de secrets en temps constant.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod ct;
mod domain;
mod error;
mod hash;
mod identity;
pub mod keyfile;
mod secret;
mod seed;

pub use ct::ct_eq;
pub use domain::Domain;
pub use error::CryptoError;
pub use hash::{sha256, sha256_domain};
pub use identity::{SigningIdentity, verify, verify_raw};
pub use secret::SecretBytes;
pub use seed::{MasterSeed, Purpose};

#[cfg(test)]
mod tests;
