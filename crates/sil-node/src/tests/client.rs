use super::support::*;
use sil_crypto::{MasterSeed, Purpose};
use sil_ledger::{Event, EventBody, Header, JobSpec, NO_PREVIOUS, SignedJobSpec};
use sil_types::JobId;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/// Requête HTTP/1.1 minimale (`Connection: close`).
pub async fn http(
    addr: SocketAddr,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: &[u8],
) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: test\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
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

/// Envoie la publication du job, signée par le requester, à l'API d'un nœud.
pub async fn publish_job(node: &TestNode, spec: JobSpec) -> JobId {
    let requester = MasterSeed::from_bytes([100; 32]).derive(Purpose::Ledger);
    let signed = SignedJobSpec::sign(&requester, spec).unwrap();
    let job = signed.verify().unwrap();
    let header = Header {
        job,
        sequence: 1,
        previous: NO_PREVIOUS,
        issued_at_ms: now_ms(),
    };
    let event = Event::sign(&requester, header, EventBody::JobPublished { spec: signed }).unwrap();
    let (status, body) = http(
        node.api.unwrap(),
        "POST",
        "/v1/events",
        Some(&node.token()),
        &event.to_bytes().unwrap(),
    )
    .await;
    assert_eq!(status, 202, "{}", String::from_utf8_lossy(&body));
    job
}
