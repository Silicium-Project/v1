//! Retenue sur la récompense des vérificateurs (ADR 0010).
//!
//! Une part de chaque récompense de vérificateur reste on-chain, au nom de sa clé. Elle est libérée
//! à la première échéance atteinte : `RELEASE_AFTER_SETTLEMENTS` règlements après son dernier vote,
//! ou `RELEASE_AFTER_SECONDS` secondes après. Une pénalité (ADR 0008) la confisque.

use crate::reputation::VERIFIER_START;

/// Part de la récompense qui reste en réserve, en pourcentage.
pub const RESERVE_PERCENT: u64 = 50;
/// Nombre de règlements (du programme entier) après le dernier vote qui libère la réserve.
pub const RELEASE_AFTER_SETTLEMENTS: u64 = 20;
/// Délai après le dernier vote qui libère la réserve : 24 h.
pub const RELEASE_AFTER_SECONDS: i64 = 24 * 60 * 60;

/// Découpe une récompense : `(part versée tout de suite, part mise en réserve)`.
pub fn split(amount: u64) -> (u64, u64) {
    let reserved = u64::try_from(u128::from(amount) * u128::from(RESERVE_PERCENT) / 100)
        .unwrap_or(amount);
    (amount - reserved, reserved)
}

/// Contenu du compte d'un vérificateur : réputation et réserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifierAccount {
    /// Score de réputation (ADR 0006).
    pub score: i64,
    /// Lamports en réserve, détenus par le compte lui-même au-dessus du loyer.
    pub reserved: u64,
    /// Valeur du compteur de règlements au dernier vote.
    pub last_vote_count: u64,
    /// Heure Unix (secondes) du dernier vote.
    pub last_vote_at: i64,
}

impl VerifierAccount {
    /// Taille du compte en octets : quatre champs de 8 octets.
    pub const LEN: usize = 32;

    /// État à l'enregistrement : score de départ, aucune réserve.
    pub fn fresh() -> Self {
        Self {
            score: VERIFIER_START,
            reserved: 0,
            last_vote_count: 0,
            last_vote_at: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut out = [0u8; Self::LEN];
        out[0..8].copy_from_slice(&self.score.to_le_bytes());
        out[8..16].copy_from_slice(&self.reserved.to_le_bytes());
        out[16..24].copy_from_slice(&self.last_vote_count.to_le_bytes());
        out[24..32].copy_from_slice(&self.last_vote_at.to_le_bytes());
        out
    }

    pub fn from_bytes(bytes: &[u8; Self::LEN]) -> Self {
        let word = |at: usize| -> [u8; 8] { bytes[at..at + 8].try_into().unwrap_or([0; 8]) };
        Self {
            score: i64::from_le_bytes(word(0)),
            reserved: u64::from_le_bytes(word(8)),
            last_vote_count: u64::from_le_bytes(word(16)),
            last_vote_at: i64::from_le_bytes(word(24)),
        }
    }

    /// Enregistre un vote : ajoute `reserved` à la réserve et repart les deux échéances.
    pub fn record_vote(&mut self, reserved: u64, count: u64, now: i64) -> Option<()> {
        self.reserved = self.reserved.checked_add(reserved)?;
        self.last_vote_count = count;
        self.last_vote_at = now;
        Some(())
    }

    /// Vrai si une réserve non nulle a atteint une de ses deux échéances.
    pub fn is_due(&self, count: u64, now: i64) -> bool {
        self.reserved > 0
            && (count.saturating_sub(self.last_vote_count) >= RELEASE_AFTER_SETTLEMENTS
                || now.saturating_sub(self.last_vote_at) >= RELEASE_AFTER_SECONDS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voted() -> VerifierAccount {
        let mut account = VerifierAccount::fresh();
        account.record_vote(50, 7, 1_000).expect("vote");
        account
    }

    #[test]
    fn half_of_a_reward_is_held_back() {
        assert_eq!(split(100), (50, 50));
        assert_eq!(split(0), (0, 0));
        assert_eq!(split(1), (1, 0), "l'arrondi favorise le versement immédiat");
        assert_eq!(split(u64::MAX).0 + split(u64::MAX).1, u64::MAX);
    }

    #[test]
    fn the_account_survives_a_byte_round_trip() {
        let account = voted();
        assert_eq!(VerifierAccount::from_bytes(&account.to_bytes()), account);
    }

    #[test]
    fn the_reserve_is_not_due_before_either_deadline() {
        let account = voted();
        assert!(!account.is_due(7 + RELEASE_AFTER_SETTLEMENTS - 1, 1_000 + RELEASE_AFTER_SECONDS - 1));
    }

    #[test]
    fn twenty_settlements_release_the_reserve() {
        assert!(voted().is_due(7 + RELEASE_AFTER_SETTLEMENTS, 1_001));
    }

    #[test]
    fn twenty_four_hours_release_the_reserve() {
        assert!(voted().is_due(8, 1_000 + RELEASE_AFTER_SECONDS));
    }

    #[test]
    fn an_empty_reserve_is_never_due() {
        assert!(!VerifierAccount::fresh().is_due(u64::MAX, i64::MAX));
    }

    #[test]
    fn a_new_vote_restarts_both_deadlines() {
        let mut account = voted();
        account.record_vote(10, 20, 90_000).expect("vote");
        assert_eq!(account.reserved, 60);
        assert!(!account.is_due(20 + RELEASE_AFTER_SETTLEMENTS - 1, 90_000 + 1));
    }
}
