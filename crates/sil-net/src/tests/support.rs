use crate::*;
use libp2p::multiaddr::Protocol;
use sil_crypto::{MasterSeed, Purpose};
use std::time::Duration;
use tokio::time::{sleep, timeout};

pub const WAIT: Duration = Duration::from_secs(20);

pub fn start_node(seed: u8, providers: Providers, limits: NetLimits) -> Network {
    let secret = MasterSeed::from_bytes([seed; 32]).derive_raw(Purpose::Network);
    let config = NetConfig {
        listen: vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
        subscribe: vec![Topic::Announce, Topic::Jobs(3)],
        limits,
        ..NetConfig::default()
    };
    start(keypair_from_secret(&secret).unwrap(), config, providers).unwrap()
}

pub fn plain_node(seed: u8) -> Network {
    start_node(seed, Providers::none(), NetLimits::default())
}

pub async fn first_listen_addr(handle: &NetworkHandle) -> Multiaddr {
    timeout(WAIT, async {
        loop {
            if let Some(address) = handle.listen_addrs().await.unwrap().first() {
                return address.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("le nœud doit écouter")
}

/// Connecte `from` à `to` et attend que les deux se voient.
pub async fn connect(from: &NetworkHandle, to: &NetworkHandle) {
    let address = first_listen_addr(to)
        .await
        .with(Protocol::P2p(to.local_peer_id()));
    from.dial(address).await.unwrap();
    timeout(WAIT, async {
        loop {
            let mine = from.peers().await.unwrap().contains(&to.local_peer_id());
            let theirs = to.peers().await.unwrap().contains(&from.local_peer_id());
            if mine && theirs {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("les nœuds doivent se connecter");
}

/// Prochain message gossip validé reçu (ignore les autres événements).
pub async fn next_gossip(net: &mut Network, wait: Duration) -> Option<(Topic, Vec<u8>)> {
    timeout(wait, async {
        loop {
            match net.events.recv().await {
                Some(NetEvent::Gossip { topic, data, .. }) => return Some((topic, data)),
                Some(_) => {}
                None => return None,
            }
        }
    })
    .await
    .ok()
    .flatten()
}
