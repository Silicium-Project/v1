use super::support::*;
use crate::*;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

/// Rejette tout message qui commence par 0xFF.
struct RejectFf;

impl MessageValidator for RejectFf {
    fn validate(&self, _topic: Topic, data: &[u8]) -> Verdict {
        if data.first() == Some(&0xFF) {
            Verdict::Reject
        } else {
            Verdict::Accept
        }
    }
}

/// Publie jusqu'à ce que le maillage soit formé (gossipsub refuse tant qu'il manque des pairs).
async fn publish_when_ready(net: &NetworkHandle, topic: Topic, data: &[u8]) {
    for _ in 0..80 {
        if net.publish(topic, data.to_vec()).await.is_ok() {
            return;
        }
        sleep(Duration::from_millis(250)).await;
    }
    panic!("publication impossible : maillage non formé");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_valid_message_is_delivered_and_an_invalid_one_is_dropped() {
    let a = plain_node(1);
    let mut b = start_node(
        2,
        Providers {
            validator: Arc::new(RejectFf),
            ..Providers::none()
        },
        NetLimits::default(),
    );
    connect(&a.handle, &b.handle).await;
    let topic = Topic::Jobs(3);

    publish_when_ready(&a.handle, topic, b"hello").await;
    assert_eq!(
        next_gossip(&mut b, WAIT).await,
        Some((topic, b"hello".to_vec()))
    );

    a.handle.publish(topic, vec![0xFF, 1, 2]).await.unwrap();
    a.handle.publish(topic, b"after".to_vec()).await.unwrap();
    assert_eq!(
        next_gossip(&mut b, WAIT).await,
        Some((topic, b"after".to_vec())),
        "le message rejeté n'est jamais livré"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversized_messages_are_refused_before_leaving_the_node() {
    let a = plain_node(1);
    let too_big = vec![0u8; NetLimits::default().max_event_message_bytes + 1];
    assert!(matches!(
        a.handle.publish(Topic::Jobs(3), too_big).await,
        Err(NetError::TooLarge { .. })
    ));
    let big_presence = vec![0u8; NetLimits::default().max_presence_bytes + 1];
    assert!(matches!(
        a.handle.publish(Topic::Presence, big_presence).await,
        Err(NetError::TooLarge { .. })
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nodes_report_connections_and_shut_down_cleanly() {
    let mut a = plain_node(1);
    let b = plain_node(2);
    connect(&a.handle, &b.handle).await;
    let connected = tokio::time::timeout(WAIT, async {
        loop {
            if let Some(NetEvent::Connected(peer)) = a.events.recv().await {
                return peer;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(connected, b.handle.local_peer_id());
    b.handle.shutdown().await;
    b.task.await.unwrap();
    assert!(b.handle.peers().await.is_err(), "après arrêt : Closed");
}
