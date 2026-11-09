//! Pénalité d'un vérificateur qui a approuvé un résultat faux (ADR 0008).
//!
//! Le requester du job la signe : c'est la seule partie lésée par un résultat faux, et elle paie le
//! job. Elle ne vise que les approbateurs d'un fragment déjà réglé, une seule fois par (fragment,
//! vérificateur). Le comité a été retiré (ADR 0007) ; le requester le remplace comme autorité de
//! sanction.

use crate::{Escrow, RuleError, reputation};
use serde::{Deserialize, Serialize};
use sil_types::PublicKey;

/// Paramètres d'une pénalité.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PenaltyArgs {
    pub fragment: u32,
    pub verifier: PublicKey,
}

impl Escrow {
    /// Pénalise `args.verifier` pour le fragment `args.fragment`. `signer` : la clé qui a signé la
    /// transaction (doit être le requester). `standing` : score actuel du vérificateur (`None` : pas
    /// de compte, donc rien à pénaliser). Retourne le nouveau score.
    pub fn penalize(
        &mut self,
        args: &PenaltyArgs,
        signer: &PublicKey,
        standing: Option<i64>,
    ) -> Result<i64, RuleError> {
        if *signer != self.terms.requester {
            return Err(RuleError::NotRequester);
        }
        if self.closed {
            return Err(RuleError::Refunded);
        }
        let settled = self
            .settled
            .get(&args.fragment)
            .ok_or(RuleError::BadPenalty("fragment non réglé"))?;
        if !settled.approvers.contains(&args.verifier) {
            return Err(RuleError::BadPenalty("n'a pas approuvé ce fragment"));
        }
        if self.penalized.contains(&(args.fragment, args.verifier)) {
            return Err(RuleError::BadPenalty("déjà pénalisé pour ce fragment"));
        }
        let score = standing.ok_or(RuleError::BadPenalty("pas de compte de réputation"))?;
        self.penalized.insert((args.fragment, args.verifier));
        Ok(reputation::penalized(score))
    }
}
