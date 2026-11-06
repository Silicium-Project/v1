use crate::TokenBucket;
use std::collections::BTreeMap;

/// Un seau par clé (pair, auteur…), nombre de clés borné.
///
/// Quand la table est pleine, les seaux revenus à plein (équivalents à un seau neuf) sont
/// purgés. S'il n'y a toujours pas de place, la **nouvelle** clé est refusée : un attaquant
/// qui multiplie les clés ne peut pas évincer les seaux déjà épuisés d'autres clés.
#[derive(Debug, Clone)]
pub struct KeyedLimiter<K: Ord + Clone> {
    buckets: BTreeMap<K, TokenBucket>,
    max_keys: usize,
    capacity: u32,
    refill_per_sec: u32,
}

impl<K: Ord + Clone> KeyedLimiter<K> {
    pub fn new(max_keys: usize, capacity: u32, refill_per_sec: u32) -> Self {
        Self {
            buckets: BTreeMap::new(),
            max_keys,
            capacity,
            refill_per_sec,
        }
    }

    /// Autorise une action de coût `cost` pour `key`.
    pub fn allow(&mut self, key: &K, cost: u32, now_ms: u64) -> bool {
        if !self.buckets.contains_key(key) {
            if self.buckets.len() >= self.max_keys {
                self.buckets.retain(|_, bucket| !bucket.is_full(now_ms));
            }
            if self.buckets.len() >= self.max_keys {
                return false;
            }
            self.buckets.insert(
                key.clone(),
                TokenBucket::new(self.capacity, self.refill_per_sec, now_ms),
            );
        }
        self.buckets
            .get_mut(key)
            .is_some_and(|bucket| bucket.try_take(cost, now_ms))
    }

    /// Nombre de clés suivies.
    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }
}
