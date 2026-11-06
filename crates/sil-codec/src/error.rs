use thiserror::Error;

/// Erreur d'encodage ou de décodage.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodecError {
    #[error("valeur non encodable")]
    Encode,
    #[error("entrée trop grande : {len} > {max} octets")]
    TooLarge { len: usize, max: usize },
    #[error("entrée mal formée")]
    Malformed,
    #[error("octets en trop après la valeur")]
    TrailingBytes,
    #[error("encodage non canonique")]
    NotCanonical,
}
