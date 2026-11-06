use crate::{Escrow, RuleError, terms::REFUND_GRACE_MS};
use sil_types::PublicKey;

impl Escrow {
    /// Rembourse au requester tout ce qui n'a pas été payé, puis ferme l'escrow.
    ///
    /// Possible seulement après `expires_at + REFUND_GRACE_MS`, pour que les règlements déjà
    /// signés puissent atterrir. Un requester ne peut donc pas retirer les fonds pendant que
    /// des workers travaillent. Retourne le montant à transférer.
    pub fn refund(&mut self, signer: &PublicKey, now_ms: u64) -> Result<u64, RuleError> {
        if *signer != self.terms.requester {
            return Err(RuleError::NotRequester);
        }
        if self.closed {
            return Err(RuleError::Refunded);
        }
        let opens_at = self
            .terms
            .expires_at_ms
            .checked_add(REFUND_GRACE_MS)
            .ok_or(RuleError::Overflow)?;
        if now_ms < opens_at {
            return Err(RuleError::TooEarlyToRefund);
        }
        let amount = self.balance();
        if amount == 0 {
            return Err(RuleError::NothingToRefund);
        }
        self.refunded = self
            .refunded
            .checked_add(amount)
            .ok_or(RuleError::Overflow)?;
        self.closed = true;
        Ok(amount)
    }
}
