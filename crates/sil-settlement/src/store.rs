use crate::{Outbox, OutboxConfig, SettlementError, outbox::Snapshot};
use sil_codec::{decode, encode};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

/// Plus grande taille acceptée pour un fichier d'outbox.
const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;

/// Sauvegarde de l'outbox : écriture dans un fichier temporaire, `fsync`, puis renommage.
/// Un arrêt brutal laisse l'ancienne version intacte.
pub struct OutboxFile {
    path: PathBuf,
}

fn storage(error: impl ToString) -> SettlementError {
    SettlementError::Storage(error.to_string())
}

impl OutboxFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn save(&self, outbox: &Outbox) -> Result<(), SettlementError> {
        let bytes = encode(&outbox.snapshot()).map_err(storage)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let temp = self.path.with_extension("tmp");
        let mut file = File::create(&temp).map_err(storage)?;
        file.write_all(&bytes).map_err(storage)?;
        file.sync_all().map_err(storage)?;
        drop(file);
        fs::rename(&temp, &self.path).map_err(storage)
    }

    /// Charge l'outbox, ou une outbox vide si le fichier n'existe pas. Un fichier corrompu est
    /// une **erreur** (jamais remplacé en silence par une outbox vide : V0 le faisait).
    pub fn load(&self, config: OutboxConfig) -> Result<Outbox, SettlementError> {
        match fs::read(&self.path) {
            Ok(bytes) => {
                let snapshot: Snapshot = decode(&bytes, MAX_FILE_BYTES)
                    .map_err(|e| SettlementError::Corrupt(e.to_string()))?;
                Ok(Outbox::from_snapshot(config, snapshot))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Outbox::new(config)),
            Err(error) => Err(storage(error)),
        }
    }
}
