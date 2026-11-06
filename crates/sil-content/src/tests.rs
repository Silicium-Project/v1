use crate::{CHUNK_SIZE, ContentError, ContentStore, Manifest, StoreConfig, merkle};
use sil_types::ContentId;

fn store_in(dir: &std::path::Path) -> ContentStore {
    ContentStore::open(StoreConfig::new(dir)).unwrap()
}

fn sample(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_add(seed) ^ ((i >> 18) as u8).wrapping_mul(37))
        .collect()
}

#[test]
fn small_empty_and_multi_chunk_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    for len in [0, 1, CHUNK_SIZE, CHUNK_SIZE + 1, 3 * CHUNK_SIZE + 17] {
        let bytes = sample(len, 3);
        let id = store.put_bytes(&bytes).unwrap();
        assert!(store.has(&id));
        assert_eq!(store.size(&id), Some(len as u64));
        let manifest = store.manifest(&id).unwrap();
        assert_eq!(manifest.id(), id);
        let mut joined = Vec::new();
        for index in 0..manifest.leaves.len() as u32 {
            joined.extend(store.chunk(&id, index).unwrap());
        }
        assert_eq!(joined, bytes);
    }
}

#[test]
fn identical_content_has_one_identity_and_is_stored_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let a = store.put_bytes(&sample(1000, 1)).unwrap();
    let b = store.put_bytes(&sample(1000, 1)).unwrap();
    assert_eq!(a, b);
    assert_eq!(store.used_bytes(), 1000);
    assert_ne!(a, store.put_bytes(&sample(1000, 2)).unwrap());
}

#[test]
fn the_id_commits_to_the_size() {
    let leaves = Manifest::of_bytes(&sample(10, 0)).leaves;
    let root = merkle::root(&leaves);
    assert_ne!(merkle::content_id(10, &root), merkle::content_id(11, &root));
}

#[test]
fn streaming_a_reader_matches_putting_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let bytes = sample(2 * CHUNK_SIZE + 5, 9);
    let streamed = store.put_reader(&mut bytes.as_slice()).unwrap();
    assert_eq!(streamed, Manifest::of_bytes(&bytes).id());
}

#[test]
fn oversized_objects_are_refused_while_streaming() {
    let dir = tempfile::tempdir().unwrap();
    let config = StoreConfig {
        max_object_bytes: 1000,
        ..StoreConfig::new(dir.path())
    };
    let store = ContentStore::open(config).unwrap();
    let result = store.put_bytes(&sample(5000, 0));
    assert!(matches!(result, Err(ContentError::TooLarge { .. })));
    assert_eq!(
        std::fs::read_dir(dir.path().join("tmp")).unwrap().count(),
        0
    );
}

#[test]
fn a_corrupted_chunk_on_disk_is_detected_when_read() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let id = store.put_bytes(&sample(100, 4)).unwrap();
    let data = dir
        .path()
        .join("objects")
        .join(&id.to_hex()[..2])
        .join(format!("{id}.data"));
    let mut bytes = std::fs::read(&data).unwrap();
    bytes[10] ^= 1;
    std::fs::write(&data, bytes).unwrap();
    assert!(matches!(
        store.chunk(&id, 0),
        Err(ContentError::BadChunk { index: 0 })
    ));
}

#[test]
fn the_index_is_rebuilt_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let id = store_in(dir.path()).put_bytes(&sample(500, 1)).unwrap();
    let reopened = store_in(dir.path());
    assert!(reopened.has(&id));
    assert_eq!(reopened.used_bytes(), 500);
}

#[test]
fn download_verifies_each_chunk_and_accepts_any_order() {
    let source_dir = tempfile::tempdir().unwrap();
    let target_dir = tempfile::tempdir().unwrap();
    let source = store_in(source_dir.path());
    let target = store_in(target_dir.path());
    let id = source.put_bytes(&sample(3 * CHUNK_SIZE + 9, 7)).unwrap();
    let manifest = source.manifest(&id).unwrap();

    let mut download = target.begin_download(&id, manifest).unwrap().unwrap();
    assert_eq!(download.missing(), vec![0, 1, 2, 3]);
    for index in [2u32, 0, 3] {
        download
            .write_chunk(index, &source.chunk(&id, index).unwrap())
            .unwrap();
    }
    assert!(matches!(
        download.write_chunk(1, &source.chunk(&id, 2).unwrap()),
        Err(ContentError::BadChunk { index: 1 })
    ));
    assert_eq!(download.missing(), vec![1]);
    assert!(
        matches!(
            target.begin_download(&id, source.manifest(&id).unwrap()),
            Err(ContentError::InProgress)
        ),
        "un second téléchargement du même contenu doit être refusé"
    );
    download
        .write_chunk(1, &source.chunk(&id, 1).unwrap())
        .unwrap();
    download.finish().unwrap();
    assert!(target.has(&id));
    assert_eq!(target.chunk(&id, 3).unwrap(), source.chunk(&id, 3).unwrap());
}

#[test]
fn an_incomplete_download_cannot_be_published() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let manifest = Manifest::of_bytes(&sample(CHUNK_SIZE + 1, 0));
    let download = store
        .begin_download(&manifest.id(), manifest)
        .unwrap()
        .unwrap();
    assert!(matches!(
        download.finish(),
        Err(ContentError::Incomplete { missing: 2 })
    ));
}

#[test]
fn a_download_resumes_after_a_restart() {
    let source_dir = tempfile::tempdir().unwrap();
    let target_dir = tempfile::tempdir().unwrap();
    let source = store_in(source_dir.path());
    let id = source.put_bytes(&sample(2 * CHUNK_SIZE, 5)).unwrap();
    let manifest = source.manifest(&id).unwrap();
    {
        let target = store_in(target_dir.path());
        let mut download = target
            .begin_download(&id, manifest.clone())
            .unwrap()
            .unwrap();
        download
            .write_chunk(0, &source.chunk(&id, 0).unwrap())
            .unwrap();
    }
    let target = store_in(target_dir.path());
    let mut download = target.begin_download(&id, manifest).unwrap().unwrap();
    assert_eq!(download.missing(), vec![1], "le morceau 0 est conservé");
    download
        .write_chunk(1, &source.chunk(&id, 1).unwrap())
        .unwrap();
    download.finish().unwrap();
    assert!(target.has(&id));
}

#[test]
fn a_manifest_for_another_id_or_with_wrong_leaves_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let manifest = Manifest::of_bytes(&sample(100, 0));
    let other = ContentId::from_bytes([9; 32]);
    assert!(matches!(
        store.begin_download(&other, manifest.clone()),
        Err(ContentError::WrongId)
    ));
    let mut padded = manifest.clone();
    padded.leaves.push(padded.leaves[0]);
    assert!(matches!(
        store.begin_download(&manifest.id(), padded),
        Err(ContentError::BadManifest(_))
    ));
}

#[test]
fn quota_evicts_the_least_recently_used_but_never_pinned_content() {
    let dir = tempfile::tempdir().unwrap();
    let config = StoreConfig {
        max_total_bytes: 2500,
        ..StoreConfig::new(dir.path())
    };
    let store = ContentStore::open(config).unwrap();
    let old = store.put_bytes(&sample(1000, 1)).unwrap();
    let pinned = store.put_bytes(&sample(1000, 2)).unwrap();
    store.pin(&pinned);
    store.chunk(&old, 0).unwrap();
    let fresh = store.put_bytes(&sample(1000, 3)).unwrap();
    assert!(!store.has(&old), "le moins récent non épinglé est évincé");
    assert!(store.has(&pinned) && store.has(&fresh));
}

#[test]
fn quota_refuses_when_everything_is_pinned() {
    let dir = tempfile::tempdir().unwrap();
    let config = StoreConfig {
        max_total_bytes: 1500,
        ..StoreConfig::new(dir.path())
    };
    let store = ContentStore::open(config).unwrap();
    let id = store.put_bytes(&sample(1000, 1)).unwrap();
    store.pin(&id);
    assert!(matches!(
        store.put_bytes(&sample(1000, 2)),
        Err(ContentError::QuotaExceeded)
    ));
    assert!(store.has(&id));
}

#[test]
fn download_reservations_count_against_the_quota() {
    let dir = tempfile::tempdir().unwrap();
    let config = StoreConfig {
        max_total_bytes: 1500,
        ..StoreConfig::new(dir.path())
    };
    let store = ContentStore::open(config).unwrap();
    let first = Manifest::of_bytes(&sample(1000, 1));
    let second = Manifest::of_bytes(&sample(1000, 2));
    let held = store.begin_download(&first.id(), first).unwrap().unwrap();
    assert!(matches!(
        store.begin_download(&second.id(), second.clone()),
        Err(ContentError::QuotaExceeded)
    ));
    drop(held);
    assert!(
        store
            .begin_download(&second.id(), second)
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_second_download_of_the_same_content_is_refused_until_the_first_ends() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let manifest = Manifest::of_bytes(&sample(CHUNK_SIZE + 5, 3));
    let id = manifest.id();
    let first = store
        .begin_download(&id, manifest.clone())
        .unwrap()
        .unwrap();
    assert!(matches!(
        store.begin_download(&id, manifest.clone()),
        Err(ContentError::InProgress)
    ));
    drop(first);
    assert!(store.begin_download(&id, manifest).unwrap().is_some());
}
