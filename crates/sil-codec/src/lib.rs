//! Encodage canonique : une valeur = une seule suite d'octets valide.
//!
//! `decode` refuse toute entrée qui ne se ré-encode pas à l'identique (longueurs non
//! minimales, champs réordonnés, octets en trop). Une signature ne peut donc pas être
//! rejouée sur une variante d'encodage (malléabilité). Les structures signées n'utilisent
//! ni `HashMap` ni flottants : voir docs/technical/protocol/encoding.md.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;

pub use error::CodecError;
use serde::{Serialize, de::DeserializeOwned};

/// Profondeur maximale d'imbrication acceptée au décodage.
pub const MAX_DEPTH: usize = 32;

/// Encode une valeur en CBOR.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    let mut out = Vec::new();
    ciborium::into_writer(value, &mut out).map_err(|_| CodecError::Encode)?;
    Ok(out)
}

/// Décode en refusant les entrées trop grandes ou non canoniques.
pub fn decode<T: Serialize + DeserializeOwned>(
    bytes: &[u8],
    max_len: usize,
) -> Result<T, CodecError> {
    if bytes.len() > max_len {
        return Err(CodecError::TooLarge {
            len: bytes.len(),
            max: max_len,
        });
    }
    let mut reader = bytes;
    let value: T = ciborium::de::from_reader_with_recursion_limit(&mut reader, MAX_DEPTH)
        .map_err(|_| CodecError::Malformed)?;
    if !reader.is_empty() {
        return Err(CodecError::TrailingBytes);
    }
    if encode(&value)? != bytes {
        return Err(CodecError::NotCanonical);
    }
    Ok(value)
}

#[cfg(test)]
mod tests;
