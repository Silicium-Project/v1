//! Chaînes `(auteur, job)` : séquence, lien au précédent, détection d'équivocation.

use crate::{Event, LedgerError, NO_PREVIOUS};
use sil_types::{Hash32, PublicKey};
use std::collections::{BTreeMap, BTreeSet};

/// Position d'un événement par rapport à la chaîne de son auteur.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Class {
    /// Déjà stocké, mêmes octets.
    Duplicate,
    /// Suite directe de la chaîne.
    Next,
    /// En avance : attend ses prédécesseurs.
    Future,
    /// Même séquence, autres octets : l'auteur a signé deux versions.
    Fork,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Chains {
    heads: BTreeMap<PublicKey, (u64, Hash32)>,
    by_sequence: BTreeMap<(PublicKey, u64), Hash32>,
    pub(crate) equivocators: BTreeSet<PublicKey>,
}

impl Chains {
    pub(crate) fn next_sequence(&self, author: &PublicKey) -> u64 {
        self.heads
            .get(author)
            .map_or(1, |(sequence, _)| sequence + 1)
    }

    /// Séquence et identifiant du dernier événement accepté de `author`.
    pub(crate) fn head(&self, author: &PublicKey) -> Option<(u64, Hash32)> {
        self.heads.get(author).copied()
    }

    /// Identifiant du dernier événement accepté de `author`.
    pub(crate) fn head_id(&self, author: &PublicKey) -> Option<Hash32> {
        self.heads.get(author).map(|(_, hash)| *hash)
    }

    pub(crate) fn head_hash(&self, author: &PublicKey) -> Hash32 {
        self.heads
            .get(author)
            .map_or(NO_PREVIOUS, |(_, hash)| *hash)
    }

    pub(crate) fn classify(
        &self,
        event: &Event,
        id: &Hash32,
        max_gap: u64,
    ) -> Result<Class, LedgerError> {
        if event.sequence == 0 {
            return Err(LedgerError::BadChain("séquence nulle"));
        }
        if self.equivocators.contains(&event.author) {
            return Err(LedgerError::Equivocation(event.author));
        }
        let next = self.next_sequence(&event.author);
        if event.sequence < next {
            return Ok(
                match self.by_sequence.get(&(event.author, event.sequence)) {
                    Some(known) if known == id => Class::Duplicate,
                    _ => Class::Fork,
                },
            );
        }
        if event.sequence > next {
            return if event.sequence - next > max_gap {
                Err(LedgerError::SequenceTooFar)
            } else {
                Ok(Class::Future)
            };
        }
        if event.previous != self.head_hash(&event.author) {
            return Err(LedgerError::BadChain("lien au précédent"));
        }
        Ok(Class::Next)
    }

    pub(crate) fn record(&mut self, event: &Event, id: Hash32) {
        self.heads.insert(event.author, (event.sequence, id));
        self.by_sequence.insert((event.author, event.sequence), id);
    }

    pub(crate) fn heads(&self) -> BTreeMap<PublicKey, u64> {
        self.heads
            .iter()
            .map(|(author, (sequence, _))| (*author, *sequence))
            .collect()
    }
}
