//! Configuration unique du nœud. Seul ce crate lit l'environnement.
//!
//! Ordre : valeurs par défaut < fichier TOML < variables `SIL_*`, puis validation.
//! La config ne contient aucun secret (seulement des chemins vers eux).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod env;
mod error;
mod load;
mod model;
mod validate;

pub use error::ConfigError;
pub use load::{load, load_from_str};
pub use model::{
    ChainConfig, ControlConfig, IsolationMode, LimitsConfig, LogConfig, NetworkConfig, NodeConfig,
    RenderConfig, TrustConfig,
};

#[cfg(test)]
mod tests;
