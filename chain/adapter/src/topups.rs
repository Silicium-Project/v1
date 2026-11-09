//! Mémoire des comptes déjà approvisionnés en loyer. Un bénéficiaire reçoit au plus un transfert
//! de loyer par fenêtre (la durée de vie d'un blockhash). Sans elle, deux règlements de fragments
//! partis en même temps lisent un solde nul et versent le loyer deux fois.

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Durée pendant laquelle un compte approvisionné n'est pas ré-approvisionné.
pub const WINDOW: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct TopUps {
    recent: HashMap<Pubkey, Instant>,
}

impl TopUps {
    /// Vrai si ce compte a été approvisionné dans la fenêtre. Oublie les comptes plus anciens.
    pub fn is_recent(&mut self, key: &Pubkey, now: Instant) -> bool {
        self.recent.retain(|_, at| now.duration_since(*at) < WINDOW);
        self.recent.contains_key(key)
    }

    /// Note que ces comptes viennent d'être approvisionnés.
    pub fn remember(&mut self, keys: &[Pubkey], now: Instant) {
        for key in keys {
            self.recent.insert(*key, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_topped_up_account_is_not_topped_up_again_within_the_window() {
        let mut topups = TopUps::default();
        let key = Pubkey::new_from_array([4; 32]);
        let start = Instant::now();
        assert!(!topups.is_recent(&key, start), "jamais approvisionné");
        topups.remember(&[key], start);
        assert!(
            topups.is_recent(&key, start + Duration::from_secs(30)),
            "dans la fenêtre"
        );
        assert!(
            !topups.is_recent(&key, start + WINDOW + Duration::from_secs(1)),
            "fenêtre écoulée"
        );
    }
}
