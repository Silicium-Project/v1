//! Modèle d'embedding sur disque : hash SHA-256 vérifié avant tout usage, téléchargement de secours.
//!
//! Un fichier dont le hash ne correspond pas n'est jamais utilisé. Le téléchargement s'écrit dans
//! `<dest>.part` et n'atteint `<dest>` qu'après vérification : un fichier partiel ou falsifié ne
//! remplace jamais un modèle bon.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use sha2::{Digest, Sha256};
use sil_types::Hash32;
use std::{
    fs,
    io::{self, Read},
    path::Path,
};
use thiserror::Error;

/// Erreur de mise en place d'un modèle.
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("hash du modèle incorrect")]
    Mismatch,
    #[error("téléchargement impossible : {0}")]
    Download(String),
    #[error("entrée-sortie : {0}")]
    Io(#[from] io::Error),
}

/// SHA-256 d'un fichier, lu en flux (le modèle fait plusieurs centaines de Mo).
pub fn file_sha256(path: &Path) -> io::Result<Hash32> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Hash32::from_bytes(hasher.finalize().into()))
}

/// Un hash SHA-256 écrit en 64 caractères hexadécimaux minuscules.
pub fn hash_from_hex(text: &str) -> Option<Hash32> {
    if text.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(Hash32::from_bytes(bytes))
}

/// Vrai si `path` existe et a exactement le hash attendu.
pub fn is_present(path: &Path, expected: Hash32) -> bool {
    file_sha256(path).is_ok_and(|actual| actual == expected)
}

/// Garantit que `dest` contient le modèle `expected`. Télécharge depuis `url` si absent ou faux.
pub fn ensure(url: &str, dest: &Path, expected: Hash32) -> Result<(), ModelError> {
    if is_present(dest, expected) {
        return Ok(());
    }
    let part = dest.with_extension("part");
    let response = ureq::get(url)
        .call()
        .map_err(|error| ModelError::Download(error.to_string()))?;
    {
        let mut reader = response.into_reader();
        let mut file = fs::File::create(&part)?;
        io::copy(&mut reader, &mut file)?;
        file.sync_all()?;
    }
    if file_sha256(&part)? != expected {
        let _ = fs::remove_file(&part);
        return Err(ModelError::Mismatch);
    }
    fs::rename(&part, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // SHA-256 de « abc » (vecteur de test du standard FIPS 180-2).
    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn hash_of_abc() -> Hash32 {
        let mut bytes = [0u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&ABC[index * 2..index * 2 + 2], 16).unwrap();
        }
        Hash32::from_bytes(bytes)
    }

    #[test]
    fn file_hash_matches_the_standard_vector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.bin");
        fs::write(&path, b"abc").unwrap();
        assert_eq!(file_sha256(&path).unwrap(), hash_of_abc());
    }

    #[test]
    fn a_wrong_hash_is_not_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.bin");
        fs::write(&path, b"abd").unwrap();
        assert!(!is_present(&path, hash_of_abc()));
        assert!(!is_present(&dir.path().join("absent.bin"), hash_of_abc()));
    }

    #[test]
    fn an_existing_good_file_needs_no_download() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.bin");
        fs::write(&path, b"abc").unwrap();
        // Une URL invalide : si le téléchargement était tenté, l'appel échouerait.
        assert!(ensure("http://127.0.0.1:9/absent", &path, hash_of_abc()).is_ok());
    }

    #[test]
    fn a_missing_file_with_no_source_fails_without_writing_dest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.onnx");
        let result = ensure("http://127.0.0.1:9/absent", &path, hash_of_abc());
        assert!(matches!(result, Err(ModelError::Download(_))));
        assert!(!path.exists());
    }
}
