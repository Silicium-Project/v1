//! Décisions du comité : une décision existe quand `threshold` membres distincts ont
//! émis le même vote. Comme `threshold` est une majorité stricte, deux décisions
//! contradictoires sur le même emplacement sont impossibles.

use super::index::Ev;
use crate::{Event, spec::JobSpec};
use sil_types::PublicKey;
use std::collections::BTreeMap;

pub(super) struct Winner<'a, K> {
    pub(super) key: K,
    pub(super) votes: Vec<&'a Event>,
}

fn threshold(spec: &JobSpec) -> usize {
    usize::try_from(spec.threshold).unwrap_or(usize::MAX)
}

/// Emplacement à choix unique (ex. quel claim pour la tentative k) : le **premier** vote
/// de chaque membre compte ; retourne le choix qui atteint le seuil.
pub(super) fn single_winner<'a, K: Ord + Clone>(
    events: &[Ev<'a>],
    spec: &JobSpec,
    pick: impl Fn(&'a Event) -> Option<K>,
) -> Option<Winner<'a, K>> {
    let mut first: BTreeMap<PublicKey, (K, &'a Event)> = BTreeMap::new();
    for (_, event) in events.iter().copied() {
        if !spec.committee.contains(&event.author) {
            continue;
        }
        let Some(key) = pick(event) else { continue };
        match first.get(&event.author) {
            Some((_, known)) if known.sequence <= event.sequence => {}
            _ => {
                first.insert(event.author, (key, event));
            }
        }
    }
    let mut groups: BTreeMap<K, Vec<&'a Event>> = BTreeMap::new();
    for (key, event) in first.into_values() {
        groups.entry(key).or_default().push(event);
    }
    groups
        .into_iter()
        .find(|(_, votes)| votes.len() >= threshold(spec))
        .map(|(key, votes)| Winner { key, votes })
}

/// Emplacement à choix multiples (ex. plusieurs vérificateurs) : chaque membre vote une
/// fois par choix ; retourne tous les choix qui atteignent le seuil.
pub(super) fn multi_winners<'a, K: Ord + Clone>(
    events: &[Ev<'a>],
    spec: &JobSpec,
    pick: impl Fn(&'a Event) -> Option<K>,
) -> Vec<Winner<'a, K>> {
    let mut groups: BTreeMap<K, BTreeMap<PublicKey, &'a Event>> = BTreeMap::new();
    for (_, event) in events.iter().copied() {
        if !spec.committee.contains(&event.author) {
            continue;
        }
        let Some(key) = pick(event) else { continue };
        let voters = groups.entry(key).or_default();
        match voters.get(&event.author) {
            Some(known) if known.sequence <= event.sequence => {}
            _ => {
                voters.insert(event.author, event);
            }
        }
    }
    groups
        .into_iter()
        .filter(|(_, voters)| voters.len() >= threshold(spec))
        .map(|(key, voters)| Winner {
            key,
            votes: voters.into_values().collect(),
        })
        .collect()
}

/// Heure convenue : le `threshold`-ième plus petit horodatage parmi les votes décisifs.
pub(super) fn agreed_time(votes: &[&Event], spec: &JobSpec) -> u64 {
    let mut times: Vec<u64> = votes.iter().map(|event| event.issued_at_ms).collect();
    times.sort_unstable();
    let index = threshold(spec)
        .saturating_sub(1)
        .min(times.len().saturating_sub(1));
    times.get(index).copied().unwrap_or(0)
}
