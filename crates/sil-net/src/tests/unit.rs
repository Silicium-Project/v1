use crate::{
    NetLimits, Presence, PresenceGate, ROLE_COMPUTE, ROLE_VERIFY, SHARDS, Topic,
    ratelimit::PeerLimiters, scoring,
};
use libp2p::PeerId;
use sil_types::JobId;

#[test]
fn topic_names_round_trip_and_reject_anything_else() {
    for topic in Topic::all() {
        assert_eq!(Topic::parse(&topic.name()), Some(topic));
    }
    assert_eq!(Topic::all().len(), usize::from(SHARDS) + 2);
    for bad in [
        "sil/v1/jobs/16",
        "sil/v1/jobs/05",
        "sil/v1/jobs/-1",
        "sil/v1/jobs/",
        "sil/v2/presence",
        "silicium-network",
        "",
    ] {
        assert_eq!(Topic::parse(bad), None, "{bad}");
    }
}

#[test]
fn a_job_always_maps_to_the_same_shard() {
    let job = JobId::from_bytes([0x25; 32]);
    assert_eq!(Topic::for_job(&job), Topic::Jobs(0x25 % SHARDS));
    assert_eq!(Topic::for_job(&job), Topic::for_job(&job));
}

fn presence(load: u8) -> Presence {
    Presence {
        version: 1,
        roles: ROLE_COMPUTE | ROLE_VERIFY,
        load,
        workloads: vec!["calcul/sha256".into()],
    }
}

#[test]
fn presence_is_tiny_and_validated() {
    let bytes = presence(10).to_bytes().unwrap();
    assert!(bytes.len() < 64, "{} octets", bytes.len());
    assert_eq!(Presence::from_bytes(&bytes, 512), Some(presence(10)));
    assert!(Presence::from_bytes(&bytes, 10).is_none(), "trop grand");
    assert!(
        presence(101)
            .to_bytes()
            .and_then(|b| Presence::from_bytes(&b, 512))
            .is_none()
    );
    let mut many = presence(1);
    many.workloads = vec!["w".into(); 9];
    assert!(
        many.to_bytes()
            .and_then(|b| Presence::from_bytes(&b, 512))
            .is_none()
    );
}

#[test]
fn presence_is_sent_on_change_or_after_silence_only() {
    let mut gate = PresenceGate::new(5_000, 60_000);
    assert!(gate.should_send(&presence(10), 0), "premier envoi");
    assert!(!gate.should_send(&presence(10), 10_000), "inchangé");
    assert!(
        !gate.should_send(&presence(20), 1_000),
        "changé mais trop tôt"
    );
    assert!(gate.should_send(&presence(20), 6_000), "changé");
    assert!(!gate.should_send(&presence(20), 30_000));
    assert!(gate.should_send(&presence(20), 66_000), "silence trop long");
}

#[test]
fn scoring_covers_every_topic_and_is_accepted_by_gossipsub() {
    let (params, thresholds) = scoring::params();
    assert_eq!(params.topics.len(), Topic::all().len());
    assert!(params.validate().is_ok());
    assert!(thresholds.validate().is_ok());
}

#[test]
fn per_peer_limiters_are_independent() {
    let limits = NetLimits {
        ledger_burst: 2,
        ledger_per_sec: 0,
        ..NetLimits::default()
    };
    let mut limiters = PeerLimiters::new(&limits);
    let (a, b) = (PeerId::random(), PeerId::random());
    assert!(limiters.allow_ledger(&a, 0));
    assert!(limiters.allow_ledger(&a, 0));
    assert!(!limiters.allow_ledger(&a, 0));
    assert!(limiters.allow_ledger(&b, 0));
}
