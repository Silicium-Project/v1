//! Stockage de la graine maîtresse : 32 octets bruts, mode 0600 (Unix), création atomique.
//!
//! Sous Windows le fichier hérite des ACL du dossier de données : ce dossier doit être
//! réservé à l'utilisateur du nœud (voir docs/technical/security/keys.md).

use crate::{CryptoError, MasterSeed};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

/// Charge la graine ou la crée. Ne remplace jamais un fichier existant.
pub fn load_or_create(path: &Path) -> Result<MasterSeed, CryptoError> {
    match load(path) {
        Err(CryptoError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        other => return other,
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let seed = MasterSeed::generate()?;
    let temp = path.with_extension("tmp");
    write_private(&temp, seed.as_bytes())?;
    // `hard_link` échoue si la cible existe : pas d'écrasement, pas de course.
    let linked = fs::hard_link(&temp, path);
    let _ = fs::remove_file(&temp);
    match linked {
        Ok(()) => Ok(seed),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => load(path),
        Err(error) => Err(error.into()),
    }
}

/// Charge une graine existante et vérifie ses permissions.
pub fn load(path: &Path) -> Result<MasterSeed, CryptoError> {
    check_permissions(path)?;
    let bytes = fs::read(path)?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidKeyFile(path.to_path_buf()))?;
    Ok(MasterSeed::from_bytes(array))
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), CryptoError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn check_permissions(path: &Path) -> Result<(), CryptoError> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)?.permissions().mode();
    if mode & 0o077 != 0 {
        return Err(CryptoError::InsecurePermissions(path.to_path_buf()));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_permissions(_path: &Path) -> Result<(), CryptoError> {
    Ok(())
}
