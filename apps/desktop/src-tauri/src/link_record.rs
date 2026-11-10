//! Liaison de cette machine à un compte, gardée sur le disque à côté de la clé (`link.json`).
//! Elle ne dépend pas du stockage du navigateur intégré, qui peut être perdu : la liaison reste
//! affichée tant que la clé de la machine est celle qui a été liée. Une écriture est atomique :
//! le fichier est complet ou ne change pas.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// What the UI shows about the link: which device, under which label, since when, with which key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkRecord {
    pub device_id: String,
    pub label: String,
    pub linked_at: String,
    pub public_key: String,
}

/// Where the link record of this machine lives.
#[derive(Debug, Clone)]
pub struct LinkRecordStore {
    path: PathBuf,
}

impl LinkRecordStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The saved record, or None when there is none or the file cannot be read.
    pub fn read(&self) -> Option<LinkRecord> {
        let raw = fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// Replaces the saved record. Stable error code `storage` when the disk refuses.
    pub fn write(&self, record: &LinkRecord) -> Result<(), &'static str> {
        let json = serde_json::to_string_pretty(record).map_err(|_| "storage")?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, json).map_err(|_| "storage")?;
        fs::rename(&temp, &self.path).map_err(|_| "storage")
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "tests fail loudly on purpose")]
mod tests {
    use super::*;

    fn sample() -> LinkRecord {
        LinkRecord {
            device_id: "dev-1".into(),
            label: "PC bureau".into(),
            linked_at: "2026-11-09T10:00:00Z".into(),
            public_key: "aa".repeat(32),
        }
    }

    #[test]
    fn a_saved_record_is_read_back() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = LinkRecordStore::new(dir.path().join("link.json"));
        assert_eq!(store.read(), None);
        store.write(&sample()).expect("written");
        assert_eq!(store.read(), Some(sample()));
    }

    #[test]
    fn a_newer_record_replaces_the_older_one() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = LinkRecordStore::new(dir.path().join("link.json"));
        store.write(&sample()).expect("first");
        let mut second = sample();
        second.label = "Portable".into();
        store.write(&second).expect("second");
        assert_eq!(store.read(), Some(second));
    }

    #[test]
    fn a_damaged_file_reads_as_no_record() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("link.json");
        fs::write(&path, "pas du json").expect("written");
        assert_eq!(LinkRecordStore::new(path).read(), None);
    }
}
