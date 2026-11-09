//! Format partagé entre le programme on-chain et l'adaptateur : instructions, graines des PDA,
//! message de règlement signé par les approbateurs, codes d'erreur. Rien ici ne dépend de Solana.

use crate::{Escrow, EscrowTerms, PenaltyArgs, RuleError, SettleArgs};
use bincode::Options;
use serde::{Deserialize, Serialize};
use sil_types::{JobId, PublicKey};

/// Graine du PDA de l'escrow : `[ESCROW_SEED, job_id]`.
pub const ESCROW_SEED: &[u8] = b"escrow";

/// Graine du PDA du compte de réputation d'un vérificateur : `[VERIFIER_SEED, clé]`.
pub const VERIFIER_SEED: &[u8] = b"verifier";

/// Préfixe de domaine du message de règlement : `sil/v1/settlement` ‖ `0x00` (sil-crypto).
pub const SETTLEMENT_DOMAIN: &[u8] = b"sil/v1/settlement\0";

/// Plus grand nombre de fragments qu'un escrow on-chain accepte. Chaque fragment réglé occupe
/// de la place dans le compte : au-delà, le loyer du compte devient déraisonnable.
pub const MAX_ONCHAIN_FRAGMENTS: u32 = 256;

/// Instruction du programme d'escrow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowIx {
    /// Requester : ouvre l'escrow du job et dépose `deposit`.
    Open {
        job: JobId,
        terms: EscrowTerms,
        deposit: u64,
    },
    /// Requester : ajoute des fonds.
    TopUp { amount: u64 },
    /// Règle un fragment. Les signatures des approbateurs sont dans les instructions Ed25519
    /// précompilées de la même transaction.
    Settle(SettleArgs),
    /// Requester : rembourse le solde après l'échéance et ferme l'escrow.
    Refund,
    /// Crée le compte de réputation d'un vérificateur, à `VERIFIER_START`. Payé par le signataire.
    RegisterVerifier { key: PublicKey },
    /// Requester : pénalise un approbateur d'un fragment réglé (ADR 0008). Signature de la transaction.
    Penalize(PenaltyArgs),
}

/// Octets signés par chaque approbateur pour un règlement, préfixe de domaine compris (ce que
/// vérifie le précompilé Ed25519). Disposition : `job ‖ fragment LE ‖ tentative LE ‖ résultat ‖
/// claimant`. La liste des approbateurs n'y figure pas : elle n'est connue qu'à la finalisation,
/// après les signatures (ADR 0007).
pub fn settlement_frame(job: &JobId, args: &SettleArgs) -> Vec<u8> {
    let mut out = SETTLEMENT_DOMAIN.to_vec();
    out.extend_from_slice(&settlement_message(job, args));
    out
}

/// Le message seul (sans préfixe de domaine) : `job ‖ fragment LE ‖ tentative LE ‖ résultat ‖
/// claimant`. C'est ce que `sil_crypto::Domain::Settlement` préfixe à la signature.
pub fn settlement_message(job: &JobId, args: &SettleArgs) -> Vec<u8> {
    let mut out = Vec::with_capacity(100);
    out.extend_from_slice(job.as_bytes());
    out.extend_from_slice(&args.fragment.to_le_bytes());
    out.extend_from_slice(&args.attempt.to_le_bytes());
    out.extend_from_slice(args.result.as_bytes());
    out.extend_from_slice(args.claimant.as_bytes());
    out
}

/// Format binaire des instructions et du compte d'escrow : bincode 1 (serde), entiers à largeur
/// fixe, aucun octet en trop accepté. Le programme on-chain l'utilise aussi, car le décodage
/// CBOR générique dépasse la pile autorisée en SBF.
fn codec() -> impl Options {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .reject_trailing_bytes()
}

/// Encode une instruction (données de l'instruction Solana).
pub fn encode_ix(ix: &EscrowIx) -> Result<Vec<u8>, bincode::Error> {
    codec().serialize(ix)
}

/// Décode une instruction. `max_len` borne la taille acceptée.
pub fn decode_ix(bytes: &[u8], max_len: usize) -> Result<EscrowIx, bincode::Error> {
    codec().with_limit(max_len as u64).deserialize(bytes)
}

/// Encode l'état d'un escrow (contenu du compte, sans en-tête de longueur).
pub fn encode_state(escrow: &Escrow) -> Result<Vec<u8>, bincode::Error> {
    codec().serialize(escrow)
}

/// Décode l'état d'un escrow. `max_len` borne la taille acceptée.
pub fn decode_state(bytes: &[u8], max_len: usize) -> Result<Escrow, bincode::Error> {
    codec().with_limit(max_len as u64).deserialize(bytes)
}

/// Codes d'erreur renvoyés par le programme (`Custom(code)`), dans l'ordre de `RULE_NAMES`.
pub const RULE_NAMES: [&str; 16] = [
    "seul le requester peut faire cette opération",
    "conditions invalides",
    "dépôt insuffisant",
    "l'escrow est remboursé",
    "fragment inconnu",
    "fragment déjà réglé",
    "numéro de tentative invalide",
    "vérificateurs invalides",
    "autorisation insuffisante (signatures des approbateurs)",
    "fonds insuffisants dans l'escrow",
    "remboursement trop tôt",
    "rien à rembourser",
    "dépassement de capacité",
    "instruction ou compte invalide",
    "vérificateur sous le seuil de réputation (ou sans compte)",
    "pénalité refusée",
];

/// Code numérique d'une règle violée (index dans `RULE_NAMES`).
pub fn rule_code(error: &RuleError) -> u32 {
    match error {
        RuleError::NotRequester => 0,
        RuleError::BadTerms(_) => 1,
        RuleError::DepositTooLow { .. } => 2,
        RuleError::Refunded => 3,
        RuleError::UnknownFragment => 4,
        RuleError::AlreadySettled { .. } => 5,
        RuleError::BadAttempt => 6,
        RuleError::BadApprovers(_) => 7,
        RuleError::NotAuthorized { .. } => 8,
        RuleError::Insufficient => 9,
        RuleError::TooEarlyToRefund => 10,
        RuleError::NothingToRefund => 11,
        RuleError::Overflow => 12,
        RuleError::IneligibleVerifier { .. } => 14,
        RuleError::BadPenalty(_) => 15,
    }
}

/// Code d'une erreur propre au programme (compte absent, instruction mal formée…).
pub const CODE_INVALID_ACCOUNT: u32 = 13;

/// Texte d'un code d'erreur, pour les journaux et l'outbox.
pub fn describe_code(code: u32) -> &'static str {
    RULE_NAMES
        .get(code as usize)
        .copied()
        .unwrap_or("erreur inconnue du programme")
}
