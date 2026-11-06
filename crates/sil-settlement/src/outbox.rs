use crate::{Authorization, Chain, IntentId, OutboxConfig, SettlementError, TxId, advance::step};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Étape d'un règlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// À soumettre (ou en attente de signatures du comité).
    Pending,
    Submitted {
        tx: TxId,
        at_ms: u64,
    },
    /// Lu sur la chaîne : le fragment est réglé exactement comme voulu.
    Confirmed,
    /// Définitif : le programme refuse, ou le fragment est réglé autrement.
    Rejected {
        reason: String,
    },
    /// Bloqué en attendant quelque chose d'extérieur (frais, escrow introuvable).
    Blocked {
        reason: String,
    },
}

impl Phase {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Confirmed | Self::Rejected { .. })
    }
}

/// État présenté à l'utilisateur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Pending,
    Submitted,
    /// La chaîne ne répond plus depuis longtemps : on ne sait pas si c'est réglé.
    Unknown,
    Confirmed,
    Rejected(String),
    Blocked(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub(crate) auth: Authorization,
    pub(crate) phase: Phase,
    pub(crate) failures: u32,
    pub(crate) attempts: u32,
    pub(crate) next_try_ms: u64,
    pub(crate) first_failure_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct Snapshot {
    pub(crate) entries: BTreeMap<IntentId, Entry>,
}

pub struct Outbox {
    pub(crate) config: OutboxConfig,
    pub(crate) entries: BTreeMap<IntentId, Entry>,
}

impl Outbox {
    pub fn new(config: OutboxConfig) -> Self {
        Self {
            config,
            entries: BTreeMap::new(),
        }
    }

    pub(crate) fn from_snapshot(config: OutboxConfig, snapshot: Snapshot) -> Self {
        Self {
            config,
            entries: snapshot.entries,
        }
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            entries: self.entries.clone(),
        }
    }

    /// Ajoute un règlement. **Idempotent** : un règlement déjà connu n'est pas dupliqué, ses
    /// nouvelles signatures sont fusionnées.
    pub fn enqueue(&mut self, authorization: Authorization) -> Result<IntentId, SettlementError> {
        let id = authorization.claim.id();
        if let Some(entry) = self.entries.get_mut(&id) {
            for (signer, signature) in authorization.signatures {
                let _ = entry.auth.add(signer, signature);
            }
            return Ok(id);
        }
        if self.entries.len() >= self.config.max_entries {
            return Err(SettlementError::Full);
        }
        let mut clean = Authorization::new(authorization.claim);
        for (signer, signature) in authorization.signatures {
            let _ = clean.add(signer, signature);
        }
        self.entries.insert(
            id,
            Entry {
                auth: clean,
                phase: Phase::Pending,
                failures: 0,
                attempts: 0,
                next_try_ms: 0,
                first_failure_ms: None,
            },
        );
        Ok(id)
    }

    /// Ajoute la signature d'un membre du comité à un règlement connu.
    pub fn add_signature(
        &mut self,
        id: &IntentId,
        signer: sil_types::PublicKey,
        signature: sil_types::Signature,
    ) -> Result<bool, SettlementError> {
        match self.entries.get_mut(id) {
            Some(entry) => entry.auth.add(signer, signature),
            None => Ok(false),
        }
    }

    /// Fait avancer chaque règlement d'un pas. Retourne le nombre de règlements qui ont changé
    /// d'étape. À appeler périodiquement (depuis un thread dédié) ; sans danger si répété.
    pub fn advance(&mut self, chain: &dyn Chain, now_ms: u64) -> usize {
        let ids: Vec<IntentId> = self.entries.keys().copied().collect();
        let mut changed = 0;
        for id in ids {
            if let Some(entry) = self.entries.get_mut(&id)
                && step(entry, chain, &self.config, now_ms)
            {
                changed += 1;
            }
        }
        changed
    }

    pub fn phase(&self, id: &IntentId) -> Option<&Phase> {
        self.entries.get(id).map(|entry| &entry.phase)
    }

    /// État à afficher. `Unknown` si la chaîne est injoignable depuis `unknown_after_ms`.
    pub fn status(&self, id: &IntentId, now_ms: u64) -> Option<Status> {
        let entry = self.entries.get(id)?;
        let silent = entry
            .first_failure_ms
            .is_some_and(|since| now_ms.saturating_sub(since) >= self.config.unknown_after_ms);
        Some(match &entry.phase {
            Phase::Confirmed => Status::Confirmed,
            Phase::Rejected { reason } => Status::Rejected(reason.clone()),
            _ if silent => Status::Unknown,
            Phase::Blocked { reason } => Status::Blocked(reason.clone()),
            Phase::Submitted { .. } => Status::Submitted,
            Phase::Pending => Status::Pending,
        })
    }

    /// État de chaque règlement (pour l'API et l'interface).
    pub fn report(&self, now_ms: u64) -> Vec<(IntentId, Status)> {
        self.entries
            .keys()
            .filter_map(|id| self.status(id, now_ms).map(|status| (*id, status)))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Règlements pas encore terminés.
    pub fn open_count(&self) -> usize {
        self.entries
            .values()
            .filter(|e| !e.phase.is_terminal())
            .count()
    }

    /// Règlements encore ouverts (ni confirmés ni rejetés), avec leurs signatures. Un membre du
    /// comité les rediffuse tant qu'ils ne sont pas réglés : une signature perdue au passage ne
    /// doit pas bloquer le règlement.
    pub fn open_authorizations(&self) -> Vec<Authorization> {
        self.entries
            .values()
            .filter(|entry| !entry.phase.is_terminal())
            .map(|entry| entry.auth.clone())
            .collect()
    }

    /// Oublie les règlements confirmés (à appeler après les avoir présentés à l'utilisateur).
    pub fn forget_confirmed(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| e.phase != Phase::Confirmed);
        before - self.entries.len()
    }
}
