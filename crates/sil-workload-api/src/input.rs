use crate::WorkloadError;

/// Lecture de l'entrée d'un job par plages, sans la charger entière.
/// Le nœud fournit une implémentation adossée au magasin de contenu.
pub trait InputReader {
    fn len(&self) -> u64;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Lit exactement `len` octets à partir de `start`.
    fn read(&self, start: u64, len: usize) -> Result<Vec<u8>, WorkloadError>;
}

/// Entrée en mémoire (tests, petites entrées).
#[derive(Debug, Clone, Copy)]
pub struct MemoryInput<'a>(pub &'a [u8]);

impl InputReader for MemoryInput<'_> {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read(&self, start: u64, len: usize) -> Result<Vec<u8>, WorkloadError> {
        let begin = usize::try_from(start).map_err(|_| WorkloadError::Read("début".into()))?;
        let end = begin
            .checked_add(len)
            .filter(|end| *end <= self.0.len())
            .ok_or_else(|| WorkloadError::Read("plage hors de l'entrée".into()))?;
        Ok(self.0[begin..end].to_vec())
    }
}
