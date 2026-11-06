use crate::{StoreProvider, TransferConfig, TransferError, fetch};
use sil_content::{CHUNK_SIZE, ContentStore, StoreConfig};
use sil_crypto::{MasterSeed, Purpose};
use sil_net::{
    ContentProvider, Multiaddr, NetConfig, Network, Protocol, Providers, Topic,
    keypair_from_secret, start,
};
use sil_types::ContentId;
use std::{sync::Arc, time::Duration};
use tokio::time::{sleep, timeout};

fn node(seed: u8, provider: Arc<dyn ContentProvider>) -> Network {
    let secret = MasterSeed::from_bytes([seed; 32]).derive_raw(Purpose::Network);
    let config = NetConfig {
        listen: vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
        subscribe: vec![Topic::Announce],
        ..NetConfig::default()
    };
    let providers = Providers {
        content: provider,
        ..Providers::none()
    };
    start(keypair_from_secret(&secret).unwrap(), config, providers).unwrap()
}

async fn connect(from: &Network, to: &Network) {
    let address: Multiaddr = timeout(Duration::from_secs(20), async {
        loop {
            if let Some(a) = to.handle.listen_addrs().await.unwrap().first() {
                return a.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap()
    .with(Protocol::P2p(to.handle.local_peer_id()));
    from.handle.dial(address).await.unwrap();
    timeout(Duration::from_secs(20), async {
        while !from
            .handle
            .peers()
            .await
            .unwrap()
            .contains(&to.handle.local_peer_id())
        {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
}

fn data(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8) ^ ((i >> 18) as u8).wrapping_mul(53))
        .collect()
}

fn store() -> (tempfile::TempDir, ContentStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentStore::open(StoreConfig::new(dir.path())).unwrap();
    (dir, store)
}

/// Sert le contenu d'un magasin mais falsifie le morceau 1.
struct Liar(StoreProvider);

impl ContentProvider for Liar {
    fn manifest(&self, id: &ContentId) -> Option<Vec<u8>> {
        self.0.manifest(id)
    }

    fn chunk(&self, id: &ContentId, index: u32) -> Option<Vec<u8>> {
        let mut chunk = self.0.chunk(id, index)?;
        if index == 1 {
            chunk[0] ^= 0xff;
        }
        Some(chunk)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_content_is_downloaded_and_verified_chunk_by_chunk() {
    let (_dir_b, store_b) = store();
    let bytes = data(3 * CHUNK_SIZE + 123);
    let id = store_b.put_bytes(&bytes).unwrap();
    let b = node(2, Arc::new(StoreProvider::new(store_b.clone())));
    let a = node(1, Arc::new(sil_net::NoContent));
    connect(&a, &b).await;

    let (_dir_a, store_a) = store();
    let peers = [b.handle.local_peer_id()];
    fetch(&a.handle, &store_a, &id, &peers, &TransferConfig::default())
        .await
        .unwrap();
    assert!(store_a.has(&id));
    let manifest = store_a.manifest(&id).unwrap();
    let mut joined = Vec::new();
    for index in 0..manifest.leaves.len() as u32 {
        joined.extend(store_a.chunk(&id, index).unwrap());
    }
    assert_eq!(joined, bytes);
    // Deuxième appel : déjà là, aucun accès réseau nécessaire.
    fetch(&a.handle, &store_a, &id, &[], &TransferConfig::default())
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lying_peer_is_skipped_and_the_honest_one_finishes_the_job() {
    let (_dir_b, store_b) = store();
    let id = store_b.put_bytes(&data(3 * CHUNK_SIZE)).unwrap();
    let honest = node(2, Arc::new(StoreProvider::new(store_b.clone())));
    let liar = node(3, Arc::new(Liar(StoreProvider::new(store_b))));
    let a = node(1, Arc::new(sil_net::NoContent));
    connect(&a, &honest).await;
    connect(&a, &liar).await;

    let (_dir_a, store_a) = store();
    let peers = [liar.handle.local_peer_id(), honest.handle.local_peer_id()];
    let config = TransferConfig { window: 1 };
    fetch(&a.handle, &store_a, &id, &peers, &config)
        .await
        .unwrap();
    assert!(store_a.has(&id));
    for index in 0..3 {
        store_a.chunk(&id, index).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failures_are_reported_when_no_peer_can_help() {
    let a = node(1, Arc::new(sil_net::NoContent));
    let b = node(2, Arc::new(sil_net::NoContent));
    connect(&a, &b).await;
    let (_dir, store_a) = store();
    let id = ContentId::from_bytes([5; 32]);
    let config = TransferConfig::default();

    let none = fetch(&a.handle, &store_a, &id, &[], &config).await;
    assert!(matches!(none, Err(TransferError::NoPeers)));
    let peers = [b.handle.local_peer_id()];
    let missing = fetch(&a.handle, &store_a, &id, &peers, &config).await;
    assert!(matches!(missing, Err(TransferError::Exhausted { .. })));
    assert!(!store_a.has(&id));
}
