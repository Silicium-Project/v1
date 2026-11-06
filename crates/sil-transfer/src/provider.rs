use sil_content::ContentStore;
use sil_net::ContentProvider;
use sil_types::ContentId;

/// Sert au réseau le contenu d'un `ContentStore` : lecture d'un seul morceau, vérifié
/// contre le manifeste avant envoi (un disque corrompu ne propage pas de faux morceau).
#[derive(Clone)]
pub struct StoreProvider {
    store: ContentStore,
}

impl StoreProvider {
    pub fn new(store: ContentStore) -> Self {
        Self { store }
    }
}

impl ContentProvider for StoreProvider {
    fn manifest(&self, id: &ContentId) -> Option<Vec<u8>> {
        self.store.manifest_bytes(id).ok()
    }

    fn chunk(&self, id: &ContentId, index: u32) -> Option<Vec<u8>> {
        self.store.chunk(id, index).ok()
    }
}
