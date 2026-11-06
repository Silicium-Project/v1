//! Format d'un enregistrement.

use sil_crypto::sha256;

const LEN_BYTES: usize = 4;
const CHECK_BYTES: usize = 4;
/// Plus grand enregistrement lisible (au-delà : considéré corrompu).
pub(crate) const MAX_RECORD_BYTES: usize = 256 * 1024;

fn check(payload: &[u8]) -> [u8; CHECK_BYTES] {
    let digest = sha256(payload);
    let mut out = [0u8; CHECK_BYTES];
    out.copy_from_slice(&digest.as_bytes()[..CHECK_BYTES]);
    out
}

/// Octets à écrire pour un événement encodé.
pub(crate) fn frame(payload: &[u8]) -> Vec<u8> {
    let len = u32::try_from(payload.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(LEN_BYTES + payload.len() + CHECK_BYTES);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&check(payload));
    out
}

/// Ce que donne la lecture à une position.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Next<'a> {
    Record {
        payload: &'a [u8],
        end: usize,
    },
    /// Fin exacte du segment.
    End,
    /// Enregistrement incomplet ou dont le contrôle échoue en tout dernier.
    Torn,
    /// Contrôle faux ou longueur absurde avec des données après : corruption.
    Corrupt,
}

pub(crate) fn read_next(buf: &[u8], pos: usize) -> Next<'_> {
    let rest = &buf[pos..];
    if rest.is_empty() {
        return Next::End;
    }
    let Some(len_bytes) = rest.first_chunk::<LEN_BYTES>() else {
        return Next::Torn;
    };
    let len = u32::from_le_bytes(*len_bytes) as usize;
    if len > MAX_RECORD_BYTES {
        return Next::Corrupt;
    }
    let total = LEN_BYTES + len + CHECK_BYTES;
    if rest.len() < total {
        return Next::Torn;
    }
    let payload = &rest[LEN_BYTES..LEN_BYTES + len];
    if rest[LEN_BYTES + len..total] != check(payload) {
        // Faux contrôle sur le tout dernier enregistrement = écriture interrompue.
        return if rest.len() == total {
            Next::Torn
        } else {
            Next::Corrupt
        };
    }
    Next::Record {
        payload,
        end: pos + total,
    }
}
