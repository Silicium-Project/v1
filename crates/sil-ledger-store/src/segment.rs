//! Fichiers de segments : nommage, liste, lecture complète d'un segment.

use crate::{
    StoreError,
    record::{Next, read_next},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PREFIX: &str = "seg-";
const SUFFIX: &str = ".log";

pub(crate) fn path_of(dir: &Path, number: u64) -> PathBuf {
    dir.join(format!("{PREFIX}{number:08}{SUFFIX}"))
}

/// Numéros de segments présents, croissants.
pub(crate) fn list(dir: &Path) -> Result<Vec<u64>, StoreError> {
    let mut numbers = Vec::new();
    for entry in fs::read_dir(dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let number = name
            .strip_prefix(PREFIX)
            .and_then(|rest| rest.strip_suffix(SUFFIX))
            .and_then(|digits| digits.parse::<u64>().ok());
        numbers.extend(number);
    }
    numbers.sort_unstable();
    Ok(numbers)
}

/// Résultat de la lecture d'un segment.
pub(crate) struct Scan {
    /// Longueur des données valides.
    pub(crate) good_len: u64,
    /// Une fin interrompue suit les données valides.
    pub(crate) torn: bool,
}

/// Lit un segment, passe chaque enregistrement à `on_payload`. Une fin interrompue n'est
/// tolérée que si `allow_torn` ; une corruption est toujours une erreur.
pub(crate) fn scan(
    path: &Path,
    allow_torn: bool,
    mut on_payload: impl FnMut(&[u8]) -> Result<(), StoreError>,
) -> Result<Scan, StoreError> {
    let buf = fs::read(path)?;
    let mut pos = 0;
    loop {
        match read_next(&buf, pos) {
            Next::Record { payload, end } => {
                on_payload(payload)?;
                pos = end;
            }
            Next::End => {
                return Ok(Scan {
                    good_len: pos as u64,
                    torn: false,
                });
            }
            Next::Torn if allow_torn => {
                return Ok(Scan {
                    good_len: pos as u64,
                    torn: true,
                });
            }
            Next::Torn | Next::Corrupt => {
                return Err(StoreError::Corrupt {
                    path: path.to_path_buf(),
                    offset: pos as u64,
                });
            }
        }
    }
}
