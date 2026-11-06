use thiserror::Error;

/// Erreur de construction ou de vérification d'une attestation ou d'un certificat.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AttestError {
    #[error("signature d'attestation invalide")]
    BadSignature,
    #[error("un rejet doit citer une preuve")]
    MissingEvidence,
    #[error("une approbation ne porte pas de preuve")]
    UnexpectedEvidence,
    #[error("attestation incohérente avec le certificat : {0}")]
    Mismatch(&'static str),
    #[error("le certificat ne doit contenir que des approbations")]
    NotApproval,
    #[error("vérificateur en double")]
    DuplicateVerifier,
    #[error("vérificateur non éligible")]
    IneligibleVerifier,
    #[error("vérificateur exclu (auteur du résultat)")]
    ExcludedVerifier,
    #[error("approbations insuffisantes : {have} < {need}")]
    TooFew { have: usize, need: u32 },
    #[error("trop d'approbations dans le certificat")]
    TooMany,
    #[error("seuil requis invalide")]
    InvalidRequirement,
    #[error("encodage impossible")]
    Encode,
}
