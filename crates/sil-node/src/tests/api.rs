use super::{client::*, support::*};
use sil_ledger::JobStatus;
use sil_settlement::testing::FakeChain;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_control_api_is_protected_and_never_reads_a_path() {
    let chain = Arc::new(FakeChain::new());
    let node = start(1, roles(false, false, false), sha_registry(), chain, true).await;
    let (api, token) = (node.api.unwrap(), node.token());

    assert_eq!(http(api, "GET", "/health", None, &[]).await.0, 200);
    assert_eq!(http(api, "GET", "/v1/peers", None, &[]).await.0, 401);
    assert_eq!(
        http(api, "GET", "/v1/peers", Some("faux"), &[]).await.0,
        401
    );
    assert_eq!(
        http(api, "GET", "/v1/peers", Some(&token), &[]).await.0,
        200
    );
    assert_eq!(http(api, "GET", "/v1/openapi.json", None, &[]).await.0, 200);

    // Un « chemin » envoyé est du contenu : son hash est celui des octets, pas du fichier.
    let (status, body) = http(
        api,
        "PUT",
        "/v1/content",
        Some(&token),
        b"C:\\Windows\\win.ini",
    )
    .await;
    assert_eq!(status, 201);
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["size"], 18);

    // Un événement mal formé ou non signé est refusé proprement.
    assert_eq!(
        http(api, "POST", "/v1/events", Some(&token), b"garbage")
            .await
            .0,
        400
    );
    node.node.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_restart_replays_the_ledger_from_disk() {
    let chain = Arc::new(FakeChain::new());
    let node = start(
        1,
        roles(false, false, false),
        sha_registry(),
        chain.clone(),
        true,
    )
    .await;
    let data = input_bytes();
    let spec = job_spec(
        &[ledger_key(1), ledger_key(2), ledger_key(3)],
        input_hash(&data),
    );
    let job = publish_job(&node, spec).await;
    assert!(node.node.shared().job_state(&job).is_some());
    let TestNode {
        node: running, dir, ..
    } = node;
    running.shutdown().await;

    let again = start_in(
        dir,
        1,
        roles(false, false, false),
        sha_registry(),
        chain,
        false,
    )
    .await;
    let state = again
        .node
        .shared()
        .job_state(&job)
        .expect("job rejoué depuis le disque");
    assert_eq!(state.status, JobStatus::Active);
    again.node.shutdown().await;
}
