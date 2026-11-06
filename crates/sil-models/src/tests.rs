use crate::{Catalog, ModelError};
use sil_content::{ContentStore, StoreConfig};

fn catalog_for(id: &str) -> String {
    format!(
        r#"
[[model]]
name = "e5-small"
[[model.file]]
path = "model/weights.bin"
id = "{id}"
size = 5
[[model.file]]
path = "tokenizer.json"
id = "{}"
size = 9
"#,
        "ab".repeat(32)
    )
}

#[test]
fn a_valid_catalog_is_parsed_and_pins_its_files() {
    let id = "11".repeat(32);
    let catalog = Catalog::parse(&catalog_for(&id)).unwrap();
    let model = catalog.get("e5-small").unwrap();
    assert_eq!(model.files.len(), 2);
    assert!(catalog.is_pinned(&sil_types::ContentId::from_hex(&id).unwrap()));
    assert!(!catalog.is_pinned(&sil_types::ContentId::from_bytes([0; 32])));
    assert_eq!(
        catalog.get("nope").unwrap_err(),
        ModelError::UnknownModel("nope".into())
    );
}

#[test]
fn unsafe_paths_duplicates_and_unknown_keys_are_refused() {
    let id = "11".repeat(32);
    for bad in ["../x", "/etc/passwd", "a\\b", "a/../b", "a//b", "C:x"] {
        let text = catalog_for(&id).replace("model/weights.bin", bad);
        assert!(Catalog::parse(&text).is_err(), "{bad}");
    }
    let dup = catalog_for(&id).replace("tokenizer.json", "model/weights.bin");
    assert!(Catalog::parse(&dup).is_err());
    let extra = format!("{}\nsurprise = 1\n", catalog_for(&id));
    assert!(Catalog::parse(&extra).is_err());
    let short_id = catalog_for("abc");
    assert!(Catalog::parse(&short_id).is_err());
}

#[test]
fn readiness_follows_the_content_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContentStore::open(StoreConfig::new(dir.path())).unwrap();
    let weights = store.put_bytes(b"12345").unwrap();
    let text = catalog_for(&weights.to_hex());
    let catalog = Catalog::parse(&text).unwrap();
    assert_eq!(catalog.missing("e5-small", &store).unwrap().len(), 1);
    assert!(!catalog.is_ready("e5-small", &store).unwrap());
    let tokenizer = ContentStoreExt::put_with_id(&store, b"123456789", &"ab".repeat(32));
    assert!(!tokenizer, "le contenu réel n'a pas l'identifiant épinglé");
}

/// Aide de test : vrai si le contenu stocké a l'identifiant attendu.
trait ContentStoreExt {
    fn put_with_id(&self, bytes: &[u8], expected: &str) -> bool;
}

impl ContentStoreExt for ContentStore {
    fn put_with_id(&self, bytes: &[u8], expected: &str) -> bool {
        self.put_bytes(bytes).unwrap().to_hex() == expected
    }
}
