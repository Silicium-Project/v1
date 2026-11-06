use crate::Workload;
use std::{collections::BTreeMap, sync::Arc};

/// Workloads connus d'un nœud, par identifiant. Un job dont le workload est absent est refusé.
#[derive(Clone, Default)]
pub struct Registry {
    workloads: BTreeMap<&'static str, Arc<dyn Workload>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute un workload (remplace un précédent de même identifiant).
    pub fn with(mut self, workload: Arc<dyn Workload>) -> Self {
        self.workloads.insert(workload.id(), workload);
        self
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Workload>> {
        self.workloads.get(id).cloned()
    }

    pub fn ids(&self) -> Vec<&'static str> {
        self.workloads.keys().copied().collect()
    }
}
