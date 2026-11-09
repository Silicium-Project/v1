//! Réputation dérivée du journal (ADR 0005, phase 1b). Lecture seule : rien n'est écrit, et rien
//! ne décide encore des vérificateurs.
//!
//! Tout se lit dans le journal : les observations (`observations.rs`) et l'heure à laquelle
//! elles sont jugées (`clock.rs`). Deux nœuds au même journal et aux mêmes scopes de confiance
//! obtiennent les mêmes scores, quelle que soit leur horloge locale.

mod clock;
mod observations;
#[cfg(test)]
mod tests;

use sil_ledger::Ledger;
use sil_reputation::{Reputation, ReputationConfig, from_observations};
use sil_types::PublicKey;

/// Réputation relue dans le journal, jugée à `as_of_ms` : l'heure du journal, pas l'horloge locale.
pub(crate) struct Derived {
    reputation: Reputation,
    as_of_ms: u64,
}

impl Default for Derived {
    fn default() -> Self {
        Self {
            reputation: Reputation::new(ReputationConfig::default(), Vec::<PublicKey>::new()),
            as_of_ms: 0,
        }
    }
}

impl Derived {
    /// Heure du journal à laquelle les scores sont jugés.
    pub(crate) fn as_of_ms(&self) -> u64 {
        self.as_of_ms
    }

    pub(crate) fn score(&self, who: &PublicKey) -> f64 {
        self.reputation.score(who, self.as_of_ms)
    }

    pub(crate) fn evidence(&self, who: &PublicKey) -> (f64, f64) {
        self.reputation.evidence(who, self.as_of_ms)
    }
}

/// Réputation de tout le journal, pour les requesters `trusted`.
pub(crate) fn derive(ledger: &Ledger, trusted: Vec<PublicKey>) -> Derived {
    let (observations, as_of_ms) = observations::collect(ledger);
    Derived {
        reputation: from_observations(ReputationConfig::default(), trusted, observations),
        as_of_ms,
    }
}

/// Dernière réputation calculée. Elle est reprise tant que la version du journal et les scopes
/// de confiance ne changent pas : le journal seul fixe le résultat, le cache ne change rien.
#[derive(Default)]
pub(crate) struct Cache {
    key: Option<(u64, Vec<PublicKey>)>,
    derived: Derived,
}

impl Cache {
    pub(crate) fn get(&mut self, ledger: &Ledger, trusted: &[PublicKey]) -> &Derived {
        let key = (ledger.version(), trusted.to_vec());
        if self.key.as_ref() != Some(&key) {
            self.derived = derive(ledger, key.1.clone());
            self.key = Some(key);
        }
        &self.derived
    }
}
