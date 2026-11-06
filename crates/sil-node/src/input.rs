use sil_content::{CHUNK_SIZE, ContentStore};
use sil_types::ContentId;
use sil_workload_api::{InputReader, WorkloadError};

/// Entrée d'un job lue par plages dans le magasin de contenu, morceau par morceau (chaque
/// morceau est revérifié contre le manifeste à la lecture).
pub struct StoreInput {
    store: ContentStore,
    id: ContentId,
    size: u64,
}

impl StoreInput {
    pub fn open(store: ContentStore, id: ContentId) -> Result<Self, WorkloadError> {
        let size = store
            .size(&id)
            .ok_or_else(|| WorkloadError::Read("entrée absente du magasin".into()))?;
        Ok(Self { store, id, size })
    }
}

impl InputReader for StoreInput {
    fn len(&self) -> u64 {
        self.size
    }

    fn read(&self, start: u64, len: usize) -> Result<Vec<u8>, WorkloadError> {
        let end = start
            .checked_add(len as u64)
            .filter(|end| *end <= self.size)
            .ok_or_else(|| WorkloadError::Read("plage hors de l'entrée".into()))?;
        let chunk_size = CHUNK_SIZE as u64;
        let mut out = Vec::with_capacity(len);
        let mut position = start;
        while position < end {
            let index = u32::try_from(position / chunk_size)
                .map_err(|_| WorkloadError::Read("entrée trop grande".into()))?;
            let chunk = self
                .store
                .chunk(&self.id, index)
                .map_err(|e| WorkloadError::Read(e.to_string()))?;
            let offset = (position % chunk_size) as usize;
            let take = (chunk.len() - offset).min((end - position) as usize);
            out.extend_from_slice(&chunk[offset..offset + take]);
            position += take as u64;
        }
        Ok(out)
    }
}

/// Lit un contenu entier (résultats, preuves : petits).
pub fn read_all(store: &ContentStore, id: &ContentId) -> Result<Vec<u8>, WorkloadError> {
    let input = StoreInput::open(store.clone(), *id)?;
    let len = usize::try_from(input.len()).map_err(|_| WorkloadError::Read("trop grand".into()))?;
    input.read(0, len)
}
