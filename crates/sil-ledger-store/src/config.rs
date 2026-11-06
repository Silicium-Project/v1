use std::path::PathBuf;

/// Quand forcer l'écriture sur disque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsyncPolicy {
    /// Après chaque événement : rien n'est perdu, plus lent.
    Always,
    /// Tous les `n` événements : perte possible des derniers (récupérés auprès des pairs).
    EveryN(u32),
    /// Jamais explicitement (tests).
    Never,
}

#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub dir: PathBuf,
    /// Taille à partir de laquelle on ouvre un nouveau segment.
    pub max_segment_bytes: u64,
    pub fsync: FsyncPolicy,
}

impl StoreConfig {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            max_segment_bytes: 8 * 1024 * 1024,
            fsync: FsyncPolicy::EveryN(32),
        }
    }
}
