use sil_content::ContentError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("aucun pair à interroger")]
    NoPeers,
    #[error("aucun pair n'a pu fournir le contenu (dernier échec : {last})")]
    Exhausted { last: String },
    #[error("stockage : {0}")]
    Content(#[from] ContentError),
}
