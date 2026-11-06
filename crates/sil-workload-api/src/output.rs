use serde::{Deserialize, Serialize};
use sil_crypto::sha256;
use sil_types::Hash32;

/// Fragment à calculer : un numéro et des paramètres propres au workload (octets canoniques).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentSpec {
    pub index: u32,
    pub params: Vec<u8>,
}

/// Résultat d'un fragment et engagement par blocs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Octets du résultat (stockés dans le magasin de contenu).
    pub result: Vec<u8>,
    /// Hash de chaque bloc vérifiable (vide si le workload se vérifie en entier).
    pub milestones: Vec<Hash32>,
}

impl Output {
    /// Hash publié dans l'événement `Submit` du journal.
    pub fn result_hash(&self) -> Hash32 {
        sha256(&self.result)
    }

    /// Racine d'engagement sur les blocs (publiée comme `milestones_root`).
    pub fn milestones_root(&self) -> Hash32 {
        let mut data = Vec::with_capacity(5 + self.milestones.len() * 32);
        data.push(0x10);
        data.extend_from_slice(&(self.milestones.len() as u32).to_le_bytes());
        for milestone in &self.milestones {
            data.extend_from_slice(milestone.as_bytes());
        }
        sha256(&data)
    }
}
