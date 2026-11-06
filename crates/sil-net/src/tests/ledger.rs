use super::support::*;
use crate::*;
use sil_types::{JobId, PublicKey};
use std::sync::Arc;

/// Sert `count` événements factices numérotés à partir de `after + 1`.
struct Fake {
    count: u8,
}

impl LedgerProvider for Fake {
    fn events_after(&self, _: &JobId, _: &PublicKey, after: u64, max: usize) -> Vec<Vec<u8>> {
        (after + 1..=u64::from(self.count))
            .take(max)
            .map(|n| vec![n as u8])
            .collect()
    }

    fn events_since(&self, _: &JobId, cursor: u64, max: usize) -> Vec<Vec<u8>> {
        (cursor + 1..=u64::from(self.count))
            .take(max)
            .map(|n| vec![n as u8])
            .collect()
    }
}

fn request(after: u64, max: u16) -> LedgerRequest {
    LedgerRequest::Author {
        job: JobId::from_bytes([1; 32]),
        author: PublicKey::from_bytes([2; 32]),
        after_sequence: after,
        max,
    }
}

fn cursor_request(cursor: u64, max: u16) -> LedgerRequest {
    LedgerRequest::Cursor {
        job: JobId::from_bytes([1; 32]),
        cursor,
        max,
    }
}

fn serving(count: u8, limits: NetLimits) -> Network {
    let providers = Providers {
        ledger: Arc::new(Fake { count }),
        ..Providers::none()
    };
    start_node(2, providers, limits)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn backfill_returns_the_events_after_a_sequence_within_the_cap() {
    let a = plain_node(1);
    let b = serving(5, NetLimits::default());
    connect(&a.handle, &b.handle).await;
    let peer = b.handle.local_peer_id();

    let events = a.handle.request_ledger(peer, request(2, 10)).await.unwrap();
    assert_eq!(events.into_events(), Some(vec![vec![3], vec![4], vec![5]]));

    let capped = a.handle.request_ledger(peer, request(0, 2)).await.unwrap();
    assert_eq!(capped.into_events().map(|e| e.len()), Some(2));

    let node_cap = NetLimits {
        max_ledger_events: 1,
        ..NetLimits::default()
    };
    let b2 = serving(5, node_cap);
    connect(&a.handle, &b2.handle).await;
    let answer = a
        .handle
        .request_ledger(b2.handle.local_peer_id(), request(0, 64))
        .await
        .unwrap();
    assert_eq!(
        answer.into_events().map(|e| e.len()),
        Some(1),
        "plafond du serveur"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_flood_of_requests_gets_busy_answers() {
    let a = plain_node(1);
    let limits = NetLimits {
        ledger_burst: 2,
        ledger_per_sec: 0,
        ..NetLimits::default()
    };
    let b = serving(3, limits);
    connect(&a.handle, &b.handle).await;
    let peer = b.handle.local_peer_id();
    let mut answers = Vec::new();
    for _ in 0..5 {
        answers.push(a.handle.request_ledger(peer, request(0, 8)).await.unwrap());
    }
    let served = answers
        .iter()
        .filter(|r| matches!(r, LedgerResponse::Events(_)))
        .count();
    assert_eq!(served, 2);
    assert!(answers[2..].iter().all(|r| *r == LedgerResponse::Busy));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn asking_an_unreachable_peer_fails_instead_of_hanging() {
    let a = plain_node(1);
    let result = a
        .handle
        .request_ledger(PeerId::random(), request(0, 1))
        .await;
    assert!(matches!(result, Err(NetError::Request(_))), "{result:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cursor_requests_walk_the_peers_journal_in_pages() {
    let a = plain_node(1);
    let b = serving(5, NetLimits::default());
    connect(&a.handle, &b.handle).await;
    let peer = b.handle.local_peer_id();

    let page = a
        .handle
        .request_ledger(peer, cursor_request(0, 2))
        .await
        .unwrap();
    assert_eq!(page.into_events(), Some(vec![vec![1], vec![2]]));
    let next = a
        .handle
        .request_ledger(peer, cursor_request(2, 10))
        .await
        .unwrap();
    assert_eq!(next.into_events(), Some(vec![vec![3], vec![4], vec![5]]));
    let done = a
        .handle
        .request_ledger(peer, cursor_request(5, 10))
        .await
        .unwrap();
    assert_eq!(done.into_events(), Some(Vec::new()), "à jour");
}
