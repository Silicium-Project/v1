use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub dir: PathBuf,
    /// Octets maximum stockés (objets complets + téléchargements en cours).
    pub max_total_bytes: u64,
    /// Taille maximale d'un objet.
    pub max_object_bytes: u64,
}

impl StoreConfig {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            max_total_bytes: 20 * 1024 * 1024 * 1024,
            max_object_bytes: 1024 * 1024 * 1024,
        }
    }
}
