use crate::SamplingSeed;
use sil_crypto::sha256;

/// Tire `count` indices distincts dans `0..total`, triés. Reproductible : même graine,
/// même tirage, sur toutes les machines. Les indices sont ceux dont `SHA-256(graine ‖ i)`
/// est le plus petit.
pub fn sample_indices(seed: &SamplingSeed, total: u32, count: u32) -> Vec<u32> {
    let mut ranked: Vec<(sil_types::Hash32, u32)> = (0..total)
        .map(|index| {
            let mut data = [0u8; 36];
            data[..32].copy_from_slice(seed.bytes());
            data[32..].copy_from_slice(&index.to_le_bytes());
            (sha256(&data), index)
        })
        .collect();
    ranked.sort_unstable();
    let mut chosen: Vec<u32> = ranked
        .into_iter()
        .take(count.min(total) as usize)
        .map(|(_, index)| index)
        .collect();
    chosen.sort_unstable();
    chosen
}
