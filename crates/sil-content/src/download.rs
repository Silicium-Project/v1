use crate::{
    ContentError, ContentStore, Manifest,
    merkle::{CHUNK_SIZE, leaf_hash},
    paths::{have_path, part_path},
};
use sil_types::ContentId;
use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
};

/// Téléchargement en cours : chaque morceau est vérifié avant écriture, et l'état est
/// conservé sur disque (`tmp/<id>.part` + `.have`) pour reprendre après un arrêt.
pub struct Download {
    store: ContentStore,
    id: ContentId,
    manifest: Manifest,
    file: Option<File>,
    have: Vec<bool>,
    done: bool,
}

impl Download {
    /// Ouvre ou reprend le fichier partiel. En cas d'échec, rend la taille réservée.
    pub(crate) fn open(
        store: ContentStore,
        id: ContentId,
        manifest: Manifest,
    ) -> Result<Self, (ContentError, u64)> {
        let size = manifest.size;
        Self::open_inner(store, id, manifest).map_err(|error| (error, size))
    }

    fn open_inner(
        store: ContentStore,
        id: ContentId,
        manifest: Manifest,
    ) -> Result<Self, ContentError> {
        let part = part_path(store.root(), &id);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&part)?;
        if file.metadata()?.len() != manifest.size {
            file.set_len(manifest.size)?;
        }
        let count = manifest.leaves.len();
        let mut have = vec![false; count];
        if let Ok(saved) = fs::read(have_path(store.root(), &id))
            && saved.len() == count
        {
            have = saved.iter().map(|b| *b == 1).collect();
        }
        Ok(Self {
            store,
            id,
            manifest,
            file: Some(file),
            have,
            done: false,
        })
    }

    pub fn id(&self) -> &ContentId {
        &self.id
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Morceaux pas encore reçus.
    pub fn missing(&self) -> Vec<u32> {
        self.have
            .iter()
            .enumerate()
            .filter(|(_, got)| !**got)
            .map(|(index, _)| index as u32)
            .collect()
    }

    /// Vérifie puis écrit un morceau. Un morceau faux est refusé sans rien écrire.
    pub fn write_chunk(&mut self, index: u32, bytes: &[u8]) -> Result<(), ContentError> {
        let expected = self
            .manifest
            .chunk_len(index)
            .ok_or(ContentError::OutOfRange { index })?;
        if bytes.len() != expected || leaf_hash(bytes) != self.manifest.leaves[index as usize] {
            return Err(ContentError::BadChunk { index });
        }
        let file = self
            .file
            .as_mut()
            .ok_or(ContentError::Incomplete { missing: 0 })?;
        file.seek(SeekFrom::Start(u64::from(index) * CHUNK_SIZE as u64))?;
        file.write_all(bytes)?;
        self.have[index as usize] = true;
        let flags: Vec<u8> = self.have.iter().map(|got| u8::from(*got)).collect();
        fs::write(have_path(self.store.root(), &self.id), flags)?;
        Ok(())
    }

    /// Publie l'objet quand tous les morceaux sont là.
    pub fn finish(mut self) -> Result<(), ContentError> {
        let missing = self.missing().len();
        if missing > 0 {
            return Err(ContentError::Incomplete { missing });
        }
        if let Some(file) = self.file.take() {
            file.sync_all()?;
        }
        let part = part_path(self.store.root(), &self.id);
        self.store.commit(&self.id, &self.manifest, &part)?;
        let _ = fs::remove_file(have_path(self.store.root(), &self.id));
        self.store.release(self.manifest.size);
        self.done = true;
        Ok(())
    }
}

impl Drop for Download {
    fn drop(&mut self) {
        if !self.done {
            self.store.release(self.manifest.size);
        }
        self.store.lock().end_download(&self.id);
    }
}
