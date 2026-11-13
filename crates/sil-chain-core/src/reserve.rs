//! Retenue sur la récompense des vérificateurs (ADR 0010), comptée par job et par vérificateur.
//!
//! Une part de chaque récompense de vérificateur reste en réserve. Elle est bookée par job, dans
//! `SettledFragment::held` (une entrée par approbateur, avec son jalon : votes et heure de la
//! réservation). Chaque entrée est libérée à son échéance propre : `RELEASE_AFTER_SETTLEMENTS`
//! votes propres du vérificateur après sa réservation, ou `RELEASE_AFTER_SECONDS` après.
//! Le compteur ne compte que **ses propres** votes : n'importe qui ne peut pas le faire avancer en
//! réglant d'autres jobs (correctif de revue).

/// Part de la récompense qui reste en réserve, en pourcentage.
pub const RESERVE_PERCENT: u64 = 50;
/// Nombre de votes propres du vérificateur après une réservation qui la rend due.
pub const RELEASE_AFTER_SETTLEMENTS: u64 = 20;
/// Délai après une réservation (ou le dernier vote) qui la rend due : 24 h.
pub const RELEASE_AFTER_SECONDS: i64 = 24 * 60 * 60;

/// Découpe une récompense : `(part versée tout de suite, part mise en réserve)`.
pub fn split(amount: u64) -> (u64, u64) {
    let reserved = u64::try_from(u128::from(amount) * u128::from(RESERVE_PERCENT) / 100)
        .unwrap_or(amount);
    (amount - reserved, reserved)
}

/// Compte d'un vérificateur (24 octets, little-endian) : score, réserve totale du compte, et nombre
/// total de ses votes approuvés. La réserve par job vit dans les escrows (`HeldEntry`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifierAccount {
    /// Score de réputation (ADR 0006).
    pub score: i64,
    /// Lamports détenus par le compte au titre de la réserve (somme des entrées non libérées).
    pub reserved: u64,
    /// Nombre total de règlements approuvés par ce vérificateur.
    pub votes: u64,
}

impl VerifierAccount {
    /// Taille du compte en octets : trois champs de 8 octets.
    pub const LEN: usize = 24;

    /// État à l'enregistrement : score de départ, aucune réserve, aucun vote.
    pub fn fresh() -> Self {
        Self {
            score: crate::reputation::VERIFIER_START,
            reserved: 0,
            votes: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut out = [0u8; Self::LEN];
        out[0..8].copy_from_slice(&self.score.to_le_bytes());
        out[8..16].copy_from_slice(&self.reserved.to_le_bytes());
        out[16..24].copy_from_slice(&self.votes.to_le_bytes());
        out
    }

    pub fn from_bytes(bytes: &[u8; Self::LEN]) -> Self {
        let word = |at: usize| -> [u8; 8] { bytes[at..at + 8].try_into().unwrap_or([0; 8]) };
        Self {
            score: i64::from_le_bytes(word(0)),
            reserved: u64::from_le_bytes(word(8)),
            votes: u64::from_le_bytes(word(16)),
        }
    }

    /// Compte un vote approuvé (le compteur propre) et ajoute `amount` à la réserve du compte.
    pub fn record_vote(&mut self, amount: u64) -> Option<()> {
        self.votes = self.votes.checked_add(1)?;
        self.reserved = self.reserved.checked_add(amount)?;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_of_a_reward_is_held_back() {
        assert_eq!(split(100), (50, 50));
        assert_eq!(split(0), (0, 0));
        assert_eq!(split(1), (1, 0), "l'arrondi favorise le versement immédiat");
        assert_eq!(split(u64::MAX).0 + split(u64::MAX).1, u64::MAX);
    }

    #[test]
    fn the_account_survives_a_byte_round_trip() {
        let mut account = VerifierAccount::fresh();
        account.record_vote(50).expect("vote");
        assert_eq!(VerifierAccount::from_bytes(&account.to_bytes()), account);
    }

    #[test]
    fn each_vote_counts_once_and_adds_its_reserve() {
        let mut account = VerifierAccount::fresh();
        account.record_vote(50).expect("vote");
        account.record_vote(10).expect("vote");
        assert_eq!((account.votes, account.reserved), (2, 60));
    }
}
