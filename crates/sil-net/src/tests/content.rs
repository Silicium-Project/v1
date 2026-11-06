use super::support::*;
use crate::*;
use sil_types::ContentId;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

fn known() -> ContentId {
    ContentId::from_bytes([7; 32])
}

struct Fake;

impl ContentProvider for Fake {
    fn manifest(&self, id: &ContentId) -> Option<Vec<u8>> {
        (*id == known()).then(|| b"manifest".to_vec())
    }

    fn chunk(&self, id: &ContentId, index: u32) -> Option<Vec<u8>> {
        match (*id == known(), index) {
            (true, 0) => Some(vec![9; 100_000]),
            (true, 1) => Some(vec![9; 300_000]), // au-dessus du plafond d'un morceau
            _ => None,
        }
    }
}

fn serving() -> Network {
    let providers = Providers {
        content: Arc::new(Fake),
        ..Providers::none()
    };
    start_node(2, providers, NetLimits::default())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manifests_and_chunks_are_served_and_unknown_content_is_not_found() {
    let a = plain_node(1);
    let b = serving();
    connect(&a.handle, &b.handle).await;
    let peer = b.handle.local_peer_id();
    let ask = |request| a.handle.request_content(peer, request);

    assert_eq!(
        ask(ContentRequest::Manifest { id: known() }).await.unwrap(),
        ContentResponse::Manifest(Bytes(b"manifest".to_vec()))
    );
    match ask(ContentRequest::Chunk {
        id: known(),
        index: 0,
    })
    .await
    .unwrap()
    {
        ContentResponse::Chunk(Bytes(data)) => assert_eq!(data.len(), 100_000),
        other => panic!("{other:?}"),
    }
    let unknown = ContentId::from_bytes([8; 32]);
    assert_eq!(
        ask(ContentRequest::Manifest { id: unknown }).await.unwrap(),
        ContentResponse::NotFound
    );
    assert_eq!(
        ask(ContentRequest::Chunk {
            id: known(),
            index: 1
        })
        .await
        .unwrap(),
        ContentResponse::NotFound,
        "un morceau plus grand que le plafond n'est jamais envoyé"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_provider_can_be_found_through_the_dht() {
    let a = plain_node(1);
    let b = plain_node(2);
    connect(&a.handle, &b.handle).await;
    a.handle.provide(known()).await.unwrap();
    let mut found = Vec::new();
    for _ in 0..40 {
        found = b.handle.find_providers(known()).await.unwrap();
        if !found.is_empty() {
            break;
        }
        sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(found, vec![a.handle.local_peer_id()]);
}
