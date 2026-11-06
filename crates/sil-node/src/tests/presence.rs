use super::{client::http, support::*};
use sil_settlement::testing::FakeChain;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peers_learn_each_others_roles_from_presence_messages() {
    let chain = Arc::new(FakeChain::new());
    let a = start(
        1,
        roles(true, true, false),
        sha_registry(),
        chain.clone(),
        true,
    )
    .await;
    let b = start(2, roles(false, false, true), sha_registry(), chain, false).await;
    connect_all(&[&a, &b]).await;

    let api = a.api.unwrap();
    let token = a.token();
    let mut seen = String::new();
    wait_for(20, "présence du vérificateur visible par l'API", || {
        !crate::shared::lock(&a.node.shared().work)
            .presence
            .is_empty()
    })
    .await;
    let (status, body) = http(api, "GET", "/v1/peers", Some(&token), &[]).await;
    assert_eq!(status, 200);
    seen.push_str(&String::from_utf8_lossy(&body));
    assert!(seen.contains("verify"), "{seen}");
    assert!(!seen.contains("coordinator"), "{seen}");
    assert!(seen.contains(&b.peer.to_string()), "{seen}");
}
