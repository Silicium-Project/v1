//! Moteur de référence : rendu déterministe, sans lancer de rayons. Il sert de moteur de test et
//! d'exemple du protocole du moteur externe (voir `external.rs`).

/// Pixels RGB24 des lignes `[y0, y1)` d'une image `width` pixels de large, pour la scène `scene`.
/// Même scène, mêmes octets, sur toutes les machines.
pub fn reference_pixels(scene: &[u8], width: u32, y0: u32, y1: u32) -> Vec<u8> {
    let seed = scene.iter().fold(0u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(u32::from(*byte))
    });
    let mut pixels = Vec::with_capacity((width * (y1 - y0) * 3) as usize);
    for y in y0..y1 {
        for x in 0..width {
            let v = x.wrapping_mul(7) ^ y.wrapping_mul(13) ^ seed;
            pixels.extend_from_slice(&[v as u8, (v >> 8) as u8, (v >> 16) as u8]);
        }
    }
    pixels
}
