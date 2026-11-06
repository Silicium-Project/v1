//! Index en mémoire : taille, dernier accès, épinglage, octets réservés. Éviction LRU.

use crate::ContentError;
use sil_types::ContentId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Entry {
    pub(crate) size: u64,
    pub(crate) accessed: u64,
    pub(crate) pinned: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Index {
    entries: BTreeMap<ContentId, Entry>,
    used: u64,
    reserved: u64,
    clock: u64,
    downloading: BTreeSet<ContentId>,
}

impl Index {
    pub(crate) fn contains(&self, id: &ContentId) -> bool {
        self.entries.contains_key(id)
    }

    pub(crate) fn size_of(&self, id: &ContentId) -> Option<u64> {
        self.entries.get(id).map(|e| e.size)
    }

    pub(crate) fn touch(&mut self, id: &ContentId) {
        self.clock += 1;
        let clock = self.clock;
        if let Some(entry) = self.entries.get_mut(id) {
            entry.accessed = clock;
        }
    }

    pub(crate) fn insert(&mut self, id: ContentId, size: u64) {
        self.clock += 1;
        if self.entries.contains_key(&id) {
            return;
        }
        self.used += size;
        self.entries.insert(
            id,
            Entry {
                size,
                accessed: self.clock,
                pinned: false,
            },
        );
    }

    pub(crate) fn remove(&mut self, id: &ContentId) {
        if let Some(entry) = self.entries.remove(id) {
            self.used -= entry.size;
        }
    }

    pub(crate) fn set_pinned(&mut self, id: &ContentId, pinned: bool) -> bool {
        self.entries
            .get_mut(id)
            .map(|e| e.pinned = pinned)
            .is_some()
    }

    /// Déclare un téléchargement ; faux s'il y en a déjà un pour ce contenu.
    pub(crate) fn start_download(&mut self, id: ContentId) -> bool {
        self.downloading.insert(id)
    }

    pub(crate) fn end_download(&mut self, id: &ContentId) {
        self.downloading.remove(id);
    }

    pub(crate) fn reserve(&mut self, bytes: u64) {
        self.reserved += bytes;
    }

    pub(crate) fn release(&mut self, bytes: u64) {
        self.reserved = self.reserved.saturating_sub(bytes);
    }

    pub(crate) fn used(&self) -> u64 {
        self.used
    }

    pub(crate) fn ids(&self) -> Vec<ContentId> {
        self.entries.keys().copied().collect()
    }

    /// Objets à évincer (les moins récemment utilisés, non épinglés) pour que `needed`
    /// octets tiennent sous `max_total`.
    pub(crate) fn victims(
        &self,
        needed: u64,
        max_total: u64,
    ) -> Result<Vec<ContentId>, ContentError> {
        let mut over = (self.used + self.reserved + needed).saturating_sub(max_total);
        if over == 0 {
            return Ok(Vec::new());
        }
        let mut candidates: Vec<(&ContentId, &Entry)> =
            self.entries.iter().filter(|(_, e)| !e.pinned).collect();
        candidates.sort_by_key(|(_, e)| e.accessed);
        let mut victims = Vec::new();
        for (id, entry) in candidates {
            victims.push(*id);
            over = over.saturating_sub(entry.size);
            if over == 0 {
                return Ok(victims);
            }
        }
        Err(ContentError::QuotaExceeded)
    }
}
