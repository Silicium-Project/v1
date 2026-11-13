use crate::{EventStore, FsyncPolicy, StoreConfig, StoreError, segment::path_of};
use sil_crypto::{MasterSeed, Purpose};
use sil_ledger::{Event, EventBody, Header, NO_PREVIOUS};
use sil_types::JobId;
use std::fs::OpenOptions;

fn event(sequence: u64, attempt: u32) -> Event {
    let identity = MasterSeed::from_bytes([5; 32]).derive(Purpose::Ledger);
    let header = Header {
        job: JobId::from_bytes([1; 32]),
        sequence,
        previous: NO_PREVIOUS,
        issued_at_ms: sequence,
    };
    Event::sign(
        &identity,
        header,
        EventBody::Claim { anchor_slot: 1,
            fragment: 0,
            attempt,
        },
    )
    .unwrap()
}

fn config(dir: &std::path::Path) -> StoreConfig {
    StoreConfig {
        fsync: FsyncPolicy::Never,
        ..StoreConfig::new(dir)
    }
}

fn replay(dir: &std::path::Path) -> (Vec<Event>, crate::OpenReport) {
    let mut events = Vec::new();
    let (_, report) = EventStore::open(config(dir), |e| events.push(e)).unwrap();
    (events, report)
}

#[test]
fn events_survive_a_restart_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, report) = EventStore::open(config(dir.path()), |_| {}).unwrap();
    assert_eq!(report.events_read, 0);
    let written: Vec<_> = (1..=5).map(|n| event(n, n as u32)).collect();
    for e in &written {
        store.append(e).unwrap();
    }
    store.sync().unwrap();
    drop(store);
    let (read, report) = replay(dir.path());
    assert_eq!(read, written);
    assert!(!report.repaired_tail);
}

#[test]
fn appending_after_reopen_continues_the_log() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = EventStore::open(config(dir.path()), |_| {}).unwrap();
    store.append(&event(1, 1)).unwrap();
    drop(store);
    let (mut store, _) = EventStore::open(config(dir.path()), |_| {}).unwrap();
    store.append(&event(2, 2)).unwrap();
    drop(store);
    assert_eq!(replay(dir.path()).0.len(), 2);
}

#[test]
fn a_torn_last_record_is_truncated_and_the_rest_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = EventStore::open(config(dir.path()), |_| {}).unwrap();
    store.append(&event(1, 1)).unwrap();
    store.append(&event(2, 2)).unwrap();
    store.sync().unwrap();
    drop(store);
    let path = path_of(dir.path(), 1);
    let len = std::fs::metadata(&path).unwrap().len();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(len - 5)
        .unwrap();
    let (read, report) = replay(dir.path());
    assert_eq!(read.len(), 1);
    assert!(report.repaired_tail);
    // Après réparation, un nouvel événement s'écrit proprement.
    let (mut store, _) = EventStore::open(config(dir.path()), |_| {}).unwrap();
    store.append(&event(2, 2)).unwrap();
    drop(store);
    let (read, report) = replay(dir.path());
    assert_eq!(read.len(), 2);
    assert!(!report.repaired_tail);
}

#[test]
fn corruption_in_the_middle_is_reported_and_nothing_is_modified() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = EventStore::open(config(dir.path()), |_| {}).unwrap();
    for n in 1..=3 {
        store.append(&event(n, n as u32)).unwrap();
    }
    store.sync().unwrap();
    drop(store);
    let path = path_of(dir.path(), 1);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[40] ^= 0xff;
    std::fs::write(&path, &bytes).unwrap();
    let result = EventStore::open(config(dir.path()), |_| {});
    assert!(matches!(result, Err(StoreError::Corrupt { .. })));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "aucune réparation silencieuse"
    );
}

#[test]
fn segments_rotate_and_replay_across_files() {
    let dir = tempfile::tempdir().unwrap();
    let small = StoreConfig {
        max_segment_bytes: 400,
        ..config(dir.path())
    };
    let (mut store, _) = EventStore::open(small.clone(), |_| {}).unwrap();
    for n in 1..=8 {
        store.append(&event(n, 1)).unwrap();
    }
    assert!(store.current_segment() > 1);
    drop(store);
    let mut count = 0;
    let (_, report) = EventStore::open(small, |_| count += 1).unwrap();
    assert_eq!(count, 8);
    assert!(report.segments > 1);
}

#[test]
fn a_torn_record_in_an_older_segment_is_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let small = StoreConfig {
        max_segment_bytes: 400,
        ..config(dir.path())
    };
    let (mut store, _) = EventStore::open(small.clone(), |_| {}).unwrap();
    for n in 1..=8 {
        store.append(&event(n, 1)).unwrap();
    }
    drop(store);
    let first = path_of(dir.path(), 1);
    let len = std::fs::metadata(&first).unwrap().len();
    OpenOptions::new()
        .write(true)
        .open(&first)
        .unwrap()
        .set_len(len - 3)
        .unwrap();
    assert!(matches!(
        EventStore::open(small, |_| {}),
        Err(StoreError::Corrupt { .. })
    ));
}
