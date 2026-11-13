use thiserror::Error;

/// Règle du programme violée. Toutes ces erreurs sont **définitives** (pas de nouvel essai).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleError {
    #[error("seul le requester peut faire cette opération")]
    NotRequester,
    #[error("conditions invalides : {0}")]
    BadTerms(&'static str),
    #[error("dépôt insuffisant : {have} < {need}")]
    DepositTooLow { have: u64, need: u64 },
    #[error("l'escrow est remboursé")]
    Refunded,
    #[error("fragment inconnu")]
    UnknownFragment,
    /// Déjà réglé ; `same` = avec exactement ces paramètres (le règlement voulu a eu lieu).
    #[error("fragment déjà réglé (identique : {same})")]
    AlreadySettled { same: bool },
    #[error("numéro de tentative invalide")]
    BadAttempt,
    #[error("vérificateurs invalides : {0}")]
    BadApprovers(&'static str),
    #[error("autorisation insuffisante : {have} signatures d'approbateurs sur {need}")]
    NotAuthorized { have: usize, need: u32 },
    /// Vérificateur sans compte de réputation (`None`) ou sous le seuil (`Some(score)`).
    #[error("vérificateur sous le seuil de réputation : {have:?} < {need}")]
    IneligibleVerifier { have: Option<i64>, need: i64 },
    #[error("pénalité refusée : {0}")]
    BadPenalty(&'static str),
    #[error("réserve non libérable : {0}")]
    ReserveNotDue(&'static str),
    #[error("fonds insuffisants dans l'escrow")]
    Insufficient,
    #[error("remboursement trop tôt")]
    TooEarlyToRefund,
    #[error("rien à rembourser")]
    NothingToRefund,
    #[error("dépassement de capacité")]
    Overflow,
}
