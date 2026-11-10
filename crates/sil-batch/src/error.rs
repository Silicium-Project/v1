use thiserror::Error;

/// Pourquoi un lot ou une plage est refusé.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BatchError {
    #[error("lot tronqué")]
    Truncated,
    #[error("en-tête SILBATCH absent")]
    Magic,
    #[error("version de lot non prise en charge : {0}")]
    Version(u8),
    #[error("type de média inconnu : {0}")]
    Kind(u8),
    #[error("trop d'enregistrements : {0}")]
    TooManyRecords(u32),
    #[error("enregistrement trop grand : {0} octets")]
    RecordTooLarge(u32),
    #[error("octets en trop après le dernier enregistrement")]
    TrailingBytes,
    #[error("plage de fragment invalide")]
    BadRange,
    #[error("cadre de résultat invalide")]
    BadFrame,
    #[error("option de lot invalide : {0}")]
    BadOption(&'static str),
}
