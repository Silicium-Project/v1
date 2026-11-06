//! Arbre de Merkle des morceaux. Préfixes de domaine : feuille `00`, nœud `01`, vide `02`,
//! identifiant `03` (évite qu'une feuille se fasse passer pour un nœud). Un nœud sans
//! frère est remonté tel quel (pas de duplication : pas d'ambiguïté entre listes de feuilles).

use sil_crypto::sha256;
use sil_types::{ContentId, Hash32};

/// Taille d'un morceau.
pub const CHUNK_SIZE: usize = 256 * 1024;

pub fn leaf_hash(chunk: &[u8]) -> Hash32 {
    let mut data = Vec::with_capacity(chunk.len() + 1);
    data.push(0x00);
    data.extend_from_slice(chunk);
    sha256(&data)
}

fn node_hash(left: &Hash32, right: &Hash32) -> Hash32 {
    let mut data = [0u8; 65];
    data[0] = 0x01;
    data[1..33].copy_from_slice(left.as_bytes());
    data[33..].copy_from_slice(right.as_bytes());
    sha256(&data)
}

/// Racine de Merkle d'une liste de feuilles.
pub fn root(leaves: &[Hash32]) -> Hash32 {
    if leaves.is_empty() {
        return sha256(&[0x02]);
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|pair| match pair {
                [left, right] => node_hash(left, right),
                [alone] => *alone,
                _ => unreachable!("chunks(2) donne 1 ou 2 éléments"),
            })
            .collect();
    }
    level[0]
}

/// Identifiant d'un contenu : lie la taille à la racine.
pub fn content_id(size: u64, root: &Hash32) -> ContentId {
    let mut data = [0u8; 41];
    data[0] = 0x03;
    data[1..9].copy_from_slice(&size.to_le_bytes());
    data[9..].copy_from_slice(root.as_bytes());
    ContentId::from_bytes(*sha256(&data).as_bytes())
}

/// Nombre de morceaux d'un contenu de `size` octets.
pub fn chunk_count(size: u64) -> u64 {
    size.div_ceil(CHUNK_SIZE as u64)
}
