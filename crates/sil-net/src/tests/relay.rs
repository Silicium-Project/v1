use super::support::*;
use crate::*;
use sil_crypto::{MasterSeed, Purpose};
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Nœud sur `127.0.0.1` ; `relay` : il sert de relais public.
fn relay_node(seed: u8, relay: bool, bootstrap: Vec<Multiaddr>) -> Network {
    let secret = MasterSeed::from_bytes([seed; 32]).derive_raw(Purpose::Network);
    let config = NetConfig {
        listen: vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
        bootstrap,
        subscribe: vec![Topic::Announce],
        relay_server: relay,
        ..NetConfig::default()
    };
    start(
        keypair_from_secret(&secret).unwrap(),
        config,
        Providers::none(),
    )
    .unwrap()
}

/// Publie jusqu'à ce que le maillage soit formé (gossipsub refuse tant qu'il manque des pairs).
async fn publish_when_ready(net: &NetworkHandle, data: &[u8]) {
    for _ in 0..80 {
        if net.publish(Topic::Announce, data.to_vec()).await.is_ok() {
            return;
        }
        sleep(Duration::from_millis(250)).await;
    }
    panic!("publication impossible : maillage non formé");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peers_behind_a_relay_reach_each_other_through_it() {
    let relay = relay_node(1, true, Vec::new());
    let relay_at = first_listen_addr(&relay.handle)
        .await
        .with(Protocol::P2p(relay.handle.local_peer_id()));
    // A et B ne connaissent que le relais : aucune adresse directe de l'un vers l'autre.
    let a = relay_node(2, false, vec![relay_at.clone()]);
    let mut b = relay_node(3, false, vec![relay_at.clone()]);
    let a_id = a.handle.local_peer_id();

    // B compose le circuit `relais/p2p-circuit/p2p/A` jusqu'à ce que la réservation de A soit
    // active (A la renouvelle à chaque tour tant qu'elle n'est pas confirmée).
    let circuit = relay_at
        .clone()
        .with(Protocol::P2pCircuit)
        .with(Protocol::P2p(a_id));
    timeout(WAIT, async {
        loop {
            let _ = b.handle.dial(circuit.clone()).await;
            if b.handle.peers().await.unwrap().contains(&a_id) {
                return;
            }
            sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .expect("B doit joindre A à travers le relais");

    // Le gossip traverse la connexion relayée.
    publish_when_ready(&a.handle, b"via-relay").await;
    let received = next_gossip(&mut b, WAIT).await;
    assert_eq!(received.map(|(_, data)| data), Some(b"via-relay".to_vec()));

    a.handle.shutdown().await;
    b.handle.shutdown().await;
    relay.handle.shutdown().await;
}
