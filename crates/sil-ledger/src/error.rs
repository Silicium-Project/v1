use sil_attest::AttestError;
use sil_codec::CodecError;
use sil_types::PublicKey;
use thiserror::Error;

/// Raison pour laquelle un événement ou une spec est refusé.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LedgerError {
    #[error("encodage : {0}")]
    Codec(#[from] CodecError),
    #[error("version inconnue")]
    BadVersion,
    #[error("signature invalide")]
    BadSignature,
    #[error("spec invalide : {0}")]
    InvalidSpec(&'static str),
    #[error("événement mal formé : {0}")]
    InvalidEvent(&'static str),
    #[error("job inconnu : l'événement est refusé tant que sa spec n'est pas connue")]
    UnknownJob,
    #[error("job refusé par la politique d'admission")]
    JobRejected,
    #[error("trop de jobs")]
    TooManyJobs,
    #[error("trop de jobs pour ce requester")]
    TooManyJobsForRequester,
    #[error("événement trop grand")]
    EventTooLarge,
    #[error("quota d'événements du job dépassé")]
    JobFull,
    #[error("quota d'octets du job dépassé")]
    JobBytesFull,
    #[error("quota de l'auteur dépassé pour ce job")]
    AuthorQuota,
    #[error("auteur non autorisé pour cet événement : {0}")]
    NotAuthorized(&'static str),
    #[error("chaîne invalide : {0}")]
    BadChain(&'static str),
    #[error("séquence trop en avance")]
    SequenceTooFar,
    #[error("tampon d'attente plein")]
    PendingFull,
    #[error("auteur pris en flagrant délit d'équivocation : {0}")]
    Equivocation(PublicKey),
    #[error("attestation ou certificat invalide : {0}")]
    Attestation(#[from] AttestError),
}
