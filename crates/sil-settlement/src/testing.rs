//! Chaîne simulée : applique les **vraies règles** (`sil-chain-core`) et permet d'injecter les
//! pannes qui piègent un client de règlement : 429, délai, transaction atterrie malgré une
//! erreur, transaction perdue, blockhash expiré.

use crate::{Authorization, Chain, ChainError, TxId, TxStatus};
use sil_chain_core::Escrow;
use sil_types::JobId;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, MutexGuard},
};

/// Opération sur laquelle injecter une panne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    Escrow,
    Submit,
    Status,
}

/// Qui soumet les transactions dans la chaîne simulée : reçoit les frais de règlement.
pub const FAKE_SUBMITTER: sil_types::PublicKey = sil_types::PublicKey::from_bytes([0xEE; 32]);

/// Durée de validité d'une transaction, en hauteurs de bloc.
const VALIDITY: u64 = 150;

enum Tx {
    Pending {
        auth: Authorization,
        valid_until: u64,
        dropped: bool,
    },
    Done(TxStatus),
}

#[derive(Default)]
struct Inner {
    escrows: BTreeMap<JobId, Escrow>,
    txs: BTreeMap<String, Tx>,
    height: u64,
    next: u64,
    failures: BTreeMap<Op, VecDeque<ChainError>>,
    land_then_report: Option<ChainError>,
    drop_next: bool,
    submit_calls: u32,
}

#[derive(Default)]
pub struct FakeChain {
    inner: Mutex<Inner>,
}

impl Inner {
    fn fault(&mut self, op: Op) -> Option<ChainError> {
        self.failures.get_mut(&op).and_then(VecDeque::pop_front)
    }

    /// Fait atterrir une transaction : mêmes règles que le programme.
    fn land(&mut self, auth: &Authorization) -> TxStatus {
        let Some(escrow) = self.escrows.get_mut(&auth.claim.job) else {
            return TxStatus::Failed("escrow inconnu".into());
        };
        // Sur la chaîne, ces signataires viennent du précompilé Ed25519.
        let signers = auth.approver_signers();
        // Le faux ne suit pas les comptes de réputation : chaque approbateur est éligible
        // (score de départ). Le gate réel est testé dans sil-chain-core.
        let standing =
            vec![Some(sil_chain_core::reputation::VERIFIER_START); auth.claim.approvers.len()];
        match escrow.settle(&auth.claim.args(), &signers, &FAKE_SUBMITTER, &standing) {
            Ok(_) => TxStatus::Confirmed,
            Err(error) => TxStatus::Failed(error.to_string()),
        }
    }
}

impl FakeChain {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn insert_escrow(&self, escrow: Escrow) {
        self.lock().escrows.insert(escrow.job, escrow);
    }

    /// Vue directe de l'escrow (sans passer par les pannes).
    pub fn escrow_now(&self, job: &JobId) -> Option<Escrow> {
        self.lock().escrows.get(job).cloned()
    }

    /// Les `times` prochains appels de `op` échouent avec `error`.
    pub fn fail(&self, op: Op, error: ChainError, times: u32) {
        let mut inner = self.lock();
        let queue = inner.failures.entry(op).or_default();
        queue.extend((0..times).map(|_| error.clone()));
    }

    /// La prochaine soumission **atterrit** mais l'appelant reçoit `error` (réponse perdue).
    pub fn land_but_report(&self, error: ChainError) {
        self.lock().land_then_report = Some(error);
    }

    /// La prochaine soumission est acceptée mais la transaction est perdue (n'atterrira jamais).
    pub fn drop_next_submit(&self) {
        self.lock().drop_next = true;
    }

    /// Un bloc : toutes les transactions en attente (non perdues) atterrissent.
    pub fn mine(&self) {
        let mut inner = self.lock();
        let ids: Vec<String> = inner
            .txs
            .iter()
            .filter(|(_, tx)| matches!(tx, Tx::Pending { dropped: false, .. }))
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(Tx::Pending { auth, .. }) = inner.txs.remove(&id) {
                let status = inner.land(&auth);
                inner.txs.insert(id, Tx::Done(status));
            }
        }
    }

    pub fn advance_height(&self, blocks: u64) {
        self.lock().height += blocks;
    }

    pub fn submit_calls(&self) -> u32 {
        self.lock().submit_calls
    }
}

impl Chain for FakeChain {
    fn pays_fees(&self) -> bool {
        true
    }

    fn escrow(&self, job: &JobId) -> Result<Option<Escrow>, ChainError> {
        let mut inner = self.lock();
        if let Some(error) = inner.fault(Op::Escrow) {
            return Err(error);
        }
        Ok(inner.escrows.get(job).cloned())
    }

    fn submit(&self, authorization: &Authorization) -> Result<TxId, ChainError> {
        let mut inner = self.lock();
        inner.submit_calls += 1;
        if let Some(error) = inner.fault(Op::Submit) {
            return Err(error);
        }
        if !inner.escrows.contains_key(&authorization.claim.job) {
            return Err(ChainError::Rejected("escrow inconnu".into()));
        }
        inner.next += 1;
        let id = format!("tx-{}", inner.next);
        if let Some(error) = inner.land_then_report.take() {
            let status = inner.land(authorization);
            inner.txs.insert(id, Tx::Done(status));
            return Err(error);
        }
        let dropped = std::mem::take(&mut inner.drop_next);
        let valid_until = inner.height + VALIDITY;
        inner.txs.insert(
            id.clone(),
            Tx::Pending {
                auth: authorization.clone(),
                valid_until,
                dropped,
            },
        );
        Ok(TxId(id))
    }

    fn tx_status(&self, tx: &TxId) -> Result<TxStatus, ChainError> {
        let mut inner = self.lock();
        if let Some(error) = inner.fault(Op::Status) {
            return Err(error);
        }
        let height = inner.height;
        Ok(match inner.txs.get(&tx.0) {
            Some(Tx::Pending { valid_until, .. }) if height > *valid_until => TxStatus::Expired,
            Some(Tx::Pending { .. }) => TxStatus::Pending,
            Some(Tx::Done(status)) => status.clone(),
            None => TxStatus::Expired,
        })
    }
}
