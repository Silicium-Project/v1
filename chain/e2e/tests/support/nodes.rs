//! Nœuds réels (libp2p sur localhost) : démarrage, liaison, API de contrôle.

use sil_crypto::MasterSeed;
use sil_ledger::{Event, EventBody, Header, JobSpec, NO_PREVIOUS, SignedJobSpec};
use sil_net::Protocol;
use sil_node::{ControlOptions, Node, NodeOptions, Roles};
use sil_settlement::Chain;
use sil_types::{JobId, PublicKey};
use sil_workload_api::Registry;
use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::chain::{ledger_identity, now_ms};

pub struct Running {
    pub node: Node,
    pub dir: TempDir,
    pub api: SocketAddr,
    /// Clé du ledger du nœud : c'est aussi son adresse de paiement.
    pub key: PublicKey,
    token: String,
}

/// Démarre un nœud. `requester` : seul requester dont il croit les journaux.
pub async fn start(
    seed: u8,
    roles: Roles,
    registry: Registry,
    chain: Arc<dyn Chain>,
    requester: PublicKey,
) -> Running {
    let dir = tempfile::tempdir().expect("dossier du nœud");
    let token_file = dir.path().join("control.token");
    let mut options = NodeOptions::new(dir.path());
    options.roles = roles;
    options.tick_ms = 100;
    options.trusted_requesters = vec![requester];
    options.listen = vec!["/ip4/127.0.0.1/tcp/0".parse().expect("adresse")];
    options.control = Some(ControlOptions {
        bind: "127.0.0.1:0".parse().expect("adresse"),
        token_file: token_file.clone(),
    });
    let node = Node::start(options, MasterSeed::from_bytes([seed; 32]), registry, chain)
        .await
        .expect("démarrage du nœud");
    let api = node.control_addr().expect("API de contrôle");
    let token = read_token(&token_file);
    let key = ledger_identity(seed).public();
    Running {
        node,
        dir,
        api,
        key,
        token,
    }
}

/// Jeton de soumission écrit par le nœud au démarrage.
fn read_token(file: &Path) -> String {
    let text = std::fs::read_to_string(file).expect("jeton de contrôle");
    let line = text
        .lines()
        .find(|line| line.starts_with("read,submit:"))
        .expect("jeton de soumission");
    line.split_once(':').expect("format du jeton").1.to_string()
}

/// Relie chaque nœud à tous les autres et laisse les abonnements gossip se propager.
pub async fn connect_all(nodes: &[&Running]) {
    for (i, from) in nodes.iter().enumerate() {
        for to in nodes.iter().skip(i + 1) {
            let address = loop {
                if let Some(a) = to
                    .node
                    .net()
                    .listen_addrs()
                    .await
                    .expect("adresses")
                    .first()
                {
                    break a.clone();
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            };
            let peer = to.node.net().local_peer_id();
            from.node
                .net()
                .dial(address.with(Protocol::P2p(peer)))
                .await
                .expect("liaison");
        }
    }
    for node in nodes {
        let started = std::time::Instant::now();
        while node.node.net().peers().await.expect("pairs").len() < nodes.len() - 1 {
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "pairs manquants"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    tokio::time::sleep(Duration::from_millis(2500)).await; // abonnements gossipsub
}

/// Dépose le contenu d'entrée sur le nœud (il le diffuse aux workers).
pub async fn upload(node: &Running, data: &[u8]) {
    let (status, body) = http(node.api, "PUT", "/v1/content", Some(&node.token), data).await;
    assert_eq!(status, 201, "{}", String::from_utf8_lossy(&body));
}

/// Publie la spec, signée par le requester, sur le nœud. Retourne l'identifiant du job.
pub async fn publish(
    node: &Running,
    spec: JobSpec,
    requester: &sil_crypto::SigningIdentity,
) -> JobId {
    let signed = SignedJobSpec::sign(requester, spec).expect("spec signée");
    let job = signed.verify().expect("identifiant du job");
    let header = Header {
        job,
        sequence: 1,
        previous: NO_PREVIOUS,
        issued_at_ms: now_ms(),
    };
    let event = Event::sign(requester, header, EventBody::JobPublished { spec: signed })
        .expect("événement signé");
    let (status, body) = http(
        node.api,
        "POST",
        "/v1/events",
        Some(&node.token),
        &event.to_bytes().expect("encodage"),
    )
    .await;
    assert_eq!(status, 202, "{}", String::from_utf8_lossy(&body));
    job
}

/// État vu par un nœud, lu sur son API de contrôle : job, travail fait, pairs. Sert au diagnostic
/// quand un règlement n'arrive pas.
pub async fn diagnose(node: &Running, job: &JobId) -> String {
    let mut out = String::new();
    for path in [
        format!("/v1/jobs/{}", job.to_hex()),
        "/v1/worker".to_string(),
        "/v1/peers".to_string(),
        "/v1/settlements".to_string(),
    ] {
        let (status, body) = http(node.api, "GET", &path, Some(&node.token), &[]).await;
        let text = String::from_utf8_lossy(&body);
        out += &format!(
            "  {path} -> {status} {}\n",
            text.chars().take(400).collect::<String>()
        );
    }
    out
}

/// Requête HTTP/1.1 minimale (`Connection: close`). Retourne le statut et le corps.
async fn http(
    addr: SocketAddr,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: &[u8],
) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect(addr).await.expect("connexion à l'API");
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: test\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await.expect("envoi");
    stream.write_all(body).await.expect("envoi");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("réponse");
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(raw.len());
    let status = String::from_utf8_lossy(&raw[..split])
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, raw.get(split + 4..).unwrap_or_default().to_vec())
}
