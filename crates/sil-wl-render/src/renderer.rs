use sil_workload_api::WorkloadError;

/// Moteur de rendu. Doit être **déterministe** : mêmes entrées, mêmes octets, sur toutes les
/// machines (sinon un vérificateur honnête rejetterait un résultat honnête).
pub trait BlockRenderer: Send + Sync {
    /// Rend les lignes `[y0, y1)` d'une image `width × height`, en RGB24, ligne par ligne
    /// (`width * (y1 - y0) * 3` octets).
    fn render(
        &self,
        scene: &[u8],
        width: u32,
        height: u32,
        y0: u32,
        y1: u32,
    ) -> Result<Vec<u8>, WorkloadError>;
}
