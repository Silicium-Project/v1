use std::path::PathBuf;
use thiserror::Error;

/// Erreur de chargement ou de validation de la configuration.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("fichier de configuration illisible : {0}")]
    Unreadable(PathBuf),
    #[error("configuration TOML invalide : {0}")]
    Parse(String),
    #[error("variable {name} invalide : {reason}")]
    BadEnv { name: &'static str, reason: String },
    #[error("{field} invalide : {reason}")]
    Invalid { field: &'static str, reason: String },
}
