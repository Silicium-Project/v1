use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkloadError {
    #[error("entrée invalide : {0}")]
    BadInput(String),
    #[error("paramètres de fragment invalides : {0}")]
    BadSpec(String),
    #[error("résultat hors limites : {0}")]
    OverLimit(&'static str),
    #[error("lecture de l'entrée impossible : {0}")]
    Read(String),
    #[error("exécution échouée : {0}")]
    Execution(String),
    #[error("opération non prise en charge par ce workload")]
    Unsupported,
}
