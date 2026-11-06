use std::path::PathBuf;
use thiserror::Error;

/// Erreur cryptographique ou de stockage de clé.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("source d'aléa indisponible")]
    Random,
    #[error("clé publique invalide")]
    InvalidKey,
    #[error("signature invalide")]
    BadSignature,
    #[error("fichier clé invalide : {0}")]
    InvalidKeyFile(PathBuf),
    #[error("permissions trop ouvertes sur {0} (attendu 0600)")]
    InsecurePermissions(PathBuf),
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
}
