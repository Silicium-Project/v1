use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
    #[error("contenu trop grand : {size} > {max} octets")]
    TooLarge { size: u64, max: u64 },
    #[error("quota de stockage dépassé (rien d'évinçable)")]
    QuotaExceeded,
    #[error("manifeste invalide : {0}")]
    BadManifest(&'static str),
    #[error("le manifeste ne correspond pas à l'identifiant demandé")]
    WrongId,
    #[error("morceau {index} invalide (hash ou taille)")]
    BadChunk { index: u32 },
    #[error("morceau {index} hors du contenu")]
    OutOfRange { index: u32 },
    #[error("téléchargement incomplet : {missing} morceau(x) manquant(s)")]
    Incomplete { missing: usize },
    #[error("un téléchargement de ce contenu est déjà en cours")]
    InProgress,
    #[error("contenu inconnu")]
    NotFound,
}
