//! Bornes d'horloge des événements reçus en direct (voir `ingest.rs`).

use super::ingest::{MAX_LIVE_AGE_MS, too_old_for_live};

#[test]
fn a_fresh_event_is_accepted() {
    let now = 1_000_000;
    assert!(!too_old_for_live(now, now));
    assert!(!too_old_for_live(now - MAX_LIVE_AGE_MS, now));
}

#[test]
fn an_antedated_event_is_refused() {
    let now = 1_000_000;
    assert!(too_old_for_live(now - MAX_LIVE_AGE_MS - 1, now));
    assert!(too_old_for_live(0, now));
}
