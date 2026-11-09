//! Réputation dérivée d'un ensemble d'observations, indépendante de l'ordre de réception (ADR 0005,
//! phase 1). Deux nodes qui connaissent les mêmes événements obtiennent les mêmes scores : les
//! observations sont appliquées dans l'ordre `(heure, identifiant)`, et non dans l'ordre où elles
//! arrivent.

use crate::{Outcome, Reputation, ReputationConfig};
use sil_types::PublicKey;

/// Une issue lue dans le journal : `who` est jugé à `at_ms` par le requester `scope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub scope: PublicKey,
    pub who: PublicKey,
    /// Requester du job et insiders : leurs observations ne comptent pas.
    pub insiders: Vec<PublicKey>,
    pub outcome: Outcome,
    /// Heure de l'événement qui fonde l'issue, en millisecondes.
    pub at_ms: u64,
}

/// La réputation qui résulte de `observations`, quel que soit leur ordre de réception.
pub fn from_observations(
    config: ReputationConfig,
    trusted: impl IntoIterator<Item = PublicKey>,
    mut observations: Vec<Observation>,
) -> Reputation {
    observations.sort_by(|a, b| a.at_ms.cmp(&b.at_ms).then(a.who.cmp(&b.who)));
    let mut reputation = Reputation::new(config, trusted);
    for observation in observations {
        reputation.observe(
            &observation.scope,
            &observation.who,
            &observation.insiders,
            observation.outcome,
            observation.at_ms,
        );
    }
    reputation
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u8) -> PublicKey {
        PublicKey::from_bytes([n; 32])
    }

    fn sample() -> Vec<Observation> {
        vec![
            Observation {
                scope: key(1),
                who: key(50),
                insiders: vec![key(10)],
                outcome: Outcome::Completed { reward: 10 },
                at_ms: 1_000,
            },
            Observation {
                scope: key(1),
                who: key(50),
                insiders: vec![key(10)],
                outcome: Outcome::Failed,
                at_ms: 5_000,
            },
            Observation {
                scope: key(1),
                who: key(60),
                insiders: vec![],
                outcome: Outcome::Completed { reward: 0 },
                at_ms: 2_000,
            },
            Observation {
                scope: key(1),
                who: key(60),
                insiders: vec![],
                outcome: Outcome::Completed { reward: 100 },
                at_ms: 2_000,
            },
        ]
    }

    fn scores(reputation: &Reputation) -> Vec<(f64, (f64, f64))> {
        [key(50), key(60)]
            .iter()
            .map(|who| {
                (
                    reputation.score(who, 10_000),
                    reputation.evidence(who, 10_000),
                )
            })
            .collect()
    }

    #[test]
    fn the_same_observations_give_the_same_scores_in_any_order() {
        let forward = from_observations(ReputationConfig::default(), [key(1)], sample());
        let mut reversed = sample();
        reversed.reverse();
        let backward = from_observations(ReputationConfig::default(), [key(1)], reversed);
        let mut rotated = sample();
        rotated.rotate_left(2);
        let other = from_observations(ReputationConfig::default(), [key(1)], rotated);
        assert_eq!(scores(&forward), scores(&backward));
        assert_eq!(scores(&forward), scores(&other));
    }

    #[test]
    fn a_newcomer_with_no_observation_stays_neutral() {
        let reputation = from_observations(ReputationConfig::default(), [key(1)], Vec::new());
        assert_eq!(reputation.score(&key(99), 0), 0.5);
    }

    #[test]
    fn insiders_observations_are_not_counted() {
        let mut only_insider = sample();
        only_insider.retain(|o| o.who == key(50));
        only_insider
            .iter_mut()
            .for_each(|o| o.insiders = vec![key(50)]);
        let reputation = from_observations(ReputationConfig::default(), [key(1)], only_insider);
        assert_eq!(reputation.evidence(&key(50), 10_000), (0.0, 0.0));
    }
}
