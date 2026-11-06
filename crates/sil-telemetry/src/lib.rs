//! Journalisation structurée. Aucun `println!` dans V1 : tout passe par `tracing`.
//!
//! Règle : ne jamais journaliser de secret (graine, jeton, clé privée). Les types
//! sensibles de `sil-crypto` ont un `Debug` masqué.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use thiserror::Error;
use tracing_subscriber::EnvFilter;

/// Réglages de journalisation (copiés depuis `sil-config`, sans dépendre de lui).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryConfig {
    pub level: String,
    pub json: bool,
}

/// Erreur d'initialisation.
#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("niveau de log invalide : {0}")]
    BadLevel(String),
    #[error("journalisation déjà initialisée")]
    AlreadyInitialized,
}

/// Installe le collecteur global. À appeler une seule fois, au démarrage du binaire.
pub fn init(config: &TelemetryConfig) -> Result<(), TelemetryError> {
    let filter = EnvFilter::try_new(&config.level)
        .map_err(|_| TelemetryError::BadLevel(config.level.clone()))?;
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    let result = if config.json {
        builder.json().try_init()
    } else {
        builder.try_init()
    };
    result.map_err(|_| TelemetryError::AlreadyInitialized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_level_is_refused_and_init_is_once() {
        let bad = TelemetryConfig {
            level: "[".into(),
            json: false,
        };
        assert!(matches!(init(&bad), Err(TelemetryError::BadLevel(_))));
        let good = TelemetryConfig {
            level: "info".into(),
            json: true,
        };
        assert!(init(&good).is_ok());
        assert!(matches!(
            init(&good),
            Err(TelemetryError::AlreadyInitialized)
        ));
    }
}
