use sil_ledger::LedgerError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
    /// Corruption au milieu des données : rien n'est modifié, l'opérateur doit décider.
    #[error("segment corrompu : {path} à l'octet {offset}")]
    Corrupt { path: PathBuf, offset: u64 },
    #[error("événement non enregistrable : {0}")]
    Event(#[from] LedgerError),
}
