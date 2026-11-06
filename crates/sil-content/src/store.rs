use crate::{
    ContentError, Download, Manifest, StoreConfig,
    index::Index,
    merkle::{CHUNK_SIZE, leaf_hash},
    paths::{data_path, manifest_path, object_dir, tmp_dir},
};
use sil_types::{ContentId, Hash32};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};

pub(crate) struct Inner {
    pub(crate) config: StoreConfig,
    pub(crate) index: Mutex<Index>,
    counter: AtomicU64,
}

/// Magasin de contenu, partageable entre tâches (clone bon marché).
#[derive(Clone)]
pub struct ContentStore {
    pub(crate) inner: Arc<Inner>,
}

impl ContentStore {
    /// Ouvre le magasin et reconstruit l'index depuis les manifestes (sans relire les données).
    pub fn open(config: StoreConfig) -> Result<Self, ContentError> {
        fs::create_dir_all(tmp_dir(&config.dir))?;
        fs::create_dir_all(config.dir.join("objects"))?;
        let mut index = Index::default();
        for shard in fs::read_dir(config.dir.join("objects"))? {
            for entry in fs::read_dir(shard?.path())? {
                let path = entry?.path();
                if let Some((id, size)) = Self::complete_object(&path) {
                    index.insert(id, size);
                }
            }
        }
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                index: Mutex::new(index),
                counter: AtomicU64::new(0),
            }),
        })
    }

    /// `Some((id, taille))` si `path` est un manifeste cohérent dont les données ont la bonne taille.
    fn complete_object(path: &Path) -> Option<(ContentId, u64)> {
        let name = path.file_name()?.to_str()?;
        let id = ContentId::from_hex(name.strip_suffix(".manifest")?).ok()?;
        let manifest = Manifest::from_bytes(&fs::read(path).ok()?).ok()?;
        if manifest.id() != id {
            return None;
        }
        let data = fs::metadata(path.with_extension("data")).ok()?;
        (data.len() == manifest.size).then_some((id, manifest.size))
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Index> {
        self.inner.index.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn root(&self) -> &Path {
        &self.inner.config.dir
    }

    pub(crate) fn unique_tmp(&self, prefix: &str) -> std::path::PathBuf {
        let n = self.inner.counter.fetch_add(1, Ordering::Relaxed);
        tmp_dir(self.root()).join(format!("{prefix}-{}-{n}", std::process::id()))
    }

    /// Présence d'un contenu complet : recherche en mémoire, aucune lecture disque.
    pub fn has(&self, id: &ContentId) -> bool {
        self.lock().contains(id)
    }

    pub fn size(&self, id: &ContentId) -> Option<u64> {
        self.lock().size_of(id)
    }

    pub fn used_bytes(&self) -> u64 {
        self.lock().used()
    }

    pub fn ids(&self) -> Vec<ContentId> {
        self.lock().ids()
    }

    /// Épingle un contenu (jamais évincé) ; faux s'il est inconnu.
    pub fn pin(&self, id: &ContentId) -> bool {
        self.lock().set_pinned(id, true)
    }

    pub fn unpin(&self, id: &ContentId) -> bool {
        self.lock().set_pinned(id, false)
    }

    pub fn manifest(&self, id: &ContentId) -> Result<Manifest, ContentError> {
        if !self.has(id) {
            return Err(ContentError::NotFound);
        }
        Manifest::from_bytes(&fs::read(manifest_path(self.root(), id))?)
    }

    /// Manifeste encodé, tel qu'envoyé aux pairs.
    pub fn manifest_bytes(&self, id: &ContentId) -> Result<Vec<u8>, ContentError> {
        self.manifest(id)?.to_bytes()
    }

    /// Lit un seul morceau et le vérifie contre le manifeste (disque corrompu détecté).
    pub fn chunk(&self, id: &ContentId, index: u32) -> Result<Vec<u8>, ContentError> {
        let manifest = self.manifest(id)?;
        let len = manifest
            .chunk_len(index)
            .ok_or(ContentError::OutOfRange { index })?;
        let mut file = File::open(data_path(self.root(), id))?;
        file.seek(SeekFrom::Start(u64::from(index) * CHUNK_SIZE as u64))?;
        let mut chunk = vec![0u8; len];
        file.read_exact(&mut chunk)?;
        if leaf_hash(&chunk) != manifest.leaves[index as usize] {
            return Err(ContentError::BadChunk { index });
        }
        self.lock().touch(id);
        Ok(chunk)
    }

    /// Stocke un contenu en mémoire.
    pub fn put_bytes(&self, bytes: &[u8]) -> Result<ContentId, ContentError> {
        self.put_reader(&mut &bytes[..])
    }

    /// Stocke un flux sans le charger en entier : un morceau en mémoire à la fois.
    pub fn put_reader(&self, reader: &mut impl Read) -> Result<ContentId, ContentError> {
        let tmp = self.unique_tmp("put");
        let result = self.stream_to(reader, &tmp);
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    fn stream_to(&self, reader: &mut impl Read, tmp: &Path) -> Result<ContentId, ContentError> {
        let max = self.inner.config.max_object_bytes;
        let mut file = File::create(tmp)?;
        let mut leaves: Vec<Hash32> = Vec::new();
        let mut size = 0u64;
        let mut buf = vec![0u8; CHUNK_SIZE];
        loop {
            let mut filled = 0;
            while filled < CHUNK_SIZE {
                let n = reader.read(&mut buf[filled..])?;
                if n == 0 {
                    break;
                }
                filled += n;
            }
            if filled == 0 {
                break;
            }
            size += filled as u64;
            if size > max {
                return Err(ContentError::TooLarge { size, max });
            }
            leaves.push(leaf_hash(&buf[..filled]));
            file.write_all(&buf[..filled])?;
            if filled < CHUNK_SIZE {
                break;
            }
        }
        file.sync_all()?;
        drop(file);
        let manifest = Manifest { size, leaves };
        let id = manifest.id();
        if self.has(&id) {
            fs::remove_file(tmp)?;
            self.lock().touch(&id);
            return Ok(id);
        }
        self.make_room(size)?;
        self.commit(&id, &manifest, tmp)?;
        Ok(id)
    }

    /// Évince le nécessaire pour accueillir `needed` octets.
    pub(crate) fn make_room(&self, needed: u64) -> Result<(), ContentError> {
        let victims = {
            let mut index = self.lock();
            let victims = index.victims(needed, self.inner.config.max_total_bytes)?;
            for id in &victims {
                index.remove(id);
            }
            victims
        };
        for id in victims {
            self.delete_files(&id);
        }
        Ok(())
    }

    /// Réserve la place d'un téléchargement (évince si besoin).
    pub(crate) fn reserve(&self, needed: u64) -> Result<(), ContentError> {
        let victims = {
            let mut index = self.lock();
            let victims = index.victims(needed, self.inner.config.max_total_bytes)?;
            for id in &victims {
                index.remove(id);
            }
            index.reserve(needed);
            victims
        };
        for id in victims {
            self.delete_files(&id);
        }
        Ok(())
    }

    pub(crate) fn release(&self, bytes: u64) {
        self.lock().release(bytes);
    }

    fn delete_files(&self, id: &ContentId) {
        let _ = fs::remove_file(manifest_path(self.root(), id));
        let _ = fs::remove_file(data_path(self.root(), id));
    }

    /// Supprime un contenu (même épinglé).
    pub fn remove(&self, id: &ContentId) {
        self.lock().remove(id);
        self.delete_files(id);
    }

    /// Publie un objet complet : données d'abord, manifeste ensuite (sa présence = complet).
    pub(crate) fn commit(
        &self,
        id: &ContentId,
        manifest: &Manifest,
        data_tmp: &Path,
    ) -> Result<(), ContentError> {
        fs::create_dir_all(object_dir(self.root(), id))?;
        let data = data_path(self.root(), id);
        if data.exists() {
            fs::remove_file(data_tmp)?;
        } else {
            fs::rename(data_tmp, &data)?;
        }
        let manifest_tmp = self.unique_tmp("manifest");
        fs::write(&manifest_tmp, manifest.to_bytes()?)?;
        let target = manifest_path(self.root(), id);
        if target.exists() {
            fs::remove_file(&manifest_tmp)?;
        } else {
            fs::rename(&manifest_tmp, &target)?;
        }
        self.lock().insert(*id, manifest.size);
        Ok(())
    }

    /// Prépare le téléchargement d'un contenu dont on connaît l'identifiant et le manifeste.
    /// `None` si le contenu est déjà complet.
    pub fn begin_download(
        &self,
        id: &ContentId,
        manifest: Manifest,
    ) -> Result<Option<Download>, ContentError> {
        manifest.verify(id, self.inner.config.max_object_bytes)?;
        if self.has(id) {
            return Ok(None);
        }
        if !self.lock().start_download(*id) {
            return Err(ContentError::InProgress);
        }
        if let Err(error) = self.reserve(manifest.size) {
            self.lock().end_download(id);
            return Err(error);
        }
        match Download::open(self.clone(), *id, manifest) {
            Ok(download) => Ok(Some(download)),
            Err((error, size)) => {
                self.release(size);
                self.lock().end_download(id);
                Err(error)
            }
        }
    }
}
