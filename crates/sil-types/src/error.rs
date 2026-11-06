use thiserror::Error;

/// Erreur de conversion d'un type de base.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TypeError {
    /// Chaîne hex absente, de mauvaise longueur, ou contenant des majuscules.
    #[error("hex invalide pour {kind}")]
    InvalidHex { kind: &'static str },
}
