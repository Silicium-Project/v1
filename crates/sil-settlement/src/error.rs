use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettlementError {
    #[error("l'outbox est pleine")]
    Full,
    #[error("signature d'autorisation invalide")]
    BadSignature,
    #[error("sauvegarde de l'outbox : {0}")]
    Storage(String),
    #[error("outbox illisible : {0}")]
    Corrupt(String),
}
