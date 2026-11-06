//! Ce que le réseau lit dans le nœud pour répondre aux pairs.

use crate::shared::{Shared, lock};
use sil_net::LedgerProvider;
use sil_types::{JobId, PublicKey};
use std::sync::Arc;

pub struct NodeLedgerProvider(pub Arc<Shared>);

impl LedgerProvider for NodeLedgerProvider {
    fn events_after(
        &self,
        job: &JobId,
        author: &PublicKey,
        after: u64,
        max: usize,
    ) -> Vec<Vec<u8>> {
        lock(&self.0.ledger)
            .events_after(job, author, after, max)
            .iter()
            .filter_map(|event| event.to_bytes().ok())
            .collect()
    }

    fn events_since(&self, job: &JobId, cursor: u64, max: usize) -> Vec<Vec<u8>> {
        let cursor = usize::try_from(cursor).unwrap_or(usize::MAX);
        lock(&self.0.ledger)
            .events_since(job, cursor, max)
            .iter()
            .filter_map(|event| event.to_bytes().ok())
            .collect()
    }
}
