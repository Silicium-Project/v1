use crate::{BatchError, MAX_RECORD_BYTES};

/// Cadre des résultats d'un fragment : suite de (len u32 | octets), sans en-tête.
/// Le résultat d'un job est la concaténation des cadres des fragments, dans l'ordre.
pub fn frame(parts: &[Vec<u8>]) -> Result<Vec<u8>, BatchError> {
    let mut out = Vec::with_capacity(parts.iter().map(|p| p.len() + 4).sum());
    for part in parts {
        let len = u32::try_from(part.len()).map_err(|_| BatchError::RecordTooLarge(u32::MAX))?;
        if len > MAX_RECORD_BYTES {
            return Err(BatchError::RecordTooLarge(len));
        }
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(part);
    }
    Ok(out)
}

/// Relit un cadre écrit par [`frame`]. Refuse toute longueur qui dépasse la fin.
pub fn unframe(bytes: &[u8]) -> Result<Vec<Vec<u8>>, BatchError> {
    let mut parts = Vec::new();
    let mut position = 0usize;
    while position < bytes.len() {
        let head = bytes
            .get(position..position + 4)
            .ok_or(BatchError::BadFrame)?;
        let mut word = [0u8; 4];
        word.copy_from_slice(head);
        let len = u32::from_le_bytes(word);
        if len > MAX_RECORD_BYTES {
            return Err(BatchError::RecordTooLarge(len));
        }
        position += 4;
        let end = position + len as usize;
        parts.push(
            bytes
                .get(position..end)
                .ok_or(BatchError::BadFrame)?
                .to_vec(),
        );
        position = end;
    }
    Ok(parts)
}
