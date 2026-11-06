use crate::{ReputationConfig, record::Record};
use sil_types::PublicKey;
use std::collections::{BTreeMap, BTreeSet};

/// Issue d'une tentative, observée dans un journal vérifié.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Finalisée avec des vérificateurs indépendants ; `reward` = valeur payée.
    Completed { reward: u64 },
    /// Bail expiré, abandon ou rejet.
    Failed,
}

#[derive(Debug, Clone)]
pub struct Reputation {
    config: ReputationConfig,
    trusted_scopes: BTreeSet<PublicKey>,
    records: BTreeMap<PublicKey, Record>,
}

impl Reputation {
    /// `trusted_scopes` : les requesters dont ce nœud croit les journaux.
    pub fn new(
        config: ReputationConfig,
        trusted_scopes: impl IntoIterator<Item = PublicKey>,
    ) -> Self {
        Self {
            config,
            trusted_scopes: trusted_scopes.into_iter().collect(),
            records: BTreeMap::new(),
        }
    }

    /// Ce nœud croit-il les journaux de ce requester ?
    pub fn trusts(&self, scope: &PublicKey) -> bool {
        self.trusted_scopes.contains(scope)
    }

    fn success_weight(&self, reward: u64) -> f64 {
        (1.0 + (1.0 + reward as f64).log10()).min(self.config.max_success_weight)
    }

    /// Enregistre une observation. Retourne `false` si elle est ignorée : scope inconnu,
    /// auto-évaluation (`who` est dans `insiders` : requester et comité du job), ou table pleine.
    pub fn observe(
        &mut self,
        scope: &PublicKey,
        who: &PublicKey,
        insiders: &[PublicKey],
        outcome: Outcome,
        now_ms: u64,
    ) -> bool {
        if !self.trusted_scopes.contains(scope) || who == scope || insiders.contains(who) {
            return false;
        }
        if !self.records.contains_key(who) && self.records.len() >= self.config.max_records {
            self.prune(now_ms);
            if self.records.len() >= self.config.max_records {
                return false;
            }
        }
        let half_life = self.config.half_life_ms;
        let weight = match outcome {
            Outcome::Completed { reward } => self.success_weight(reward),
            Outcome::Failed => 1.0,
        };
        let record = self
            .records
            .entry(*who)
            .or_insert_with(|| Record::new(now_ms))
            .decayed(now_ms, half_life);
        let updated = match outcome {
            Outcome::Completed { .. } => Record {
                good: record.good + weight,
                ..record
            },
            Outcome::Failed => Record {
                bad: record.bad + weight,
                ..record
            },
        };
        self.records.insert(*who, updated);
        true
    }

    fn prune(&mut self, now_ms: u64) {
        let half_life = self.config.half_life_ms;
        self.records
            .retain(|_, record| !record.decayed(now_ms, half_life).is_negligible());
    }

    fn current(&self, who: &PublicKey, now_ms: u64) -> Record {
        self.records.get(who).map_or_else(
            || Record::new(now_ms),
            |r| r.decayed(now_ms, self.config.half_life_ms),
        )
    }

    /// Moyenne a posteriori d'une loi Beta(1,1) : 0,5 pour un inconnu, vers 1 avec les succès,
    /// vers 0 avec les échecs.
    pub fn score(&self, who: &PublicKey, now_ms: u64) -> f64 {
        let record = self.current(who, now_ms);
        (record.good + 1.0) / (record.good + record.bad + 2.0)
    }

    /// Nombre de tâches simultanées confiables à ce worker : 1 en probation, puis croissant
    /// avec ses succès récents (et nul tant que ses échecs dominent largement).
    pub fn max_parallel(&self, who: &PublicKey, now_ms: u64) -> u32 {
        let record = self.current(who, now_ms);
        if record.bad > record.good + 3.0 {
            return 0;
        }
        let earned = record.good.floor().max(0.0) as u32;
        (1 + earned).min(self.config.max_parallel)
    }

    pub fn tracked(&self) -> usize {
        self.records.len()
    }
}
