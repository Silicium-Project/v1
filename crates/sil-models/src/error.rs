use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModelError {
    #[error("catalogue illisible : {0}")]
    Parse(String),
    #[error("catalogue invalide : {0}")]
    Invalid(String),
    #[error("modèle inconnu : {0}")]
    UnknownModel(String),
}
