use crate::{
    band::{Band, Header},
    renderer::BlockRenderer,
    workload::{RenderWorkload, block_rows, blocks_in},
};
use sil_crypto::sha256;
use sil_types::Hash32;
use sil_workload_api::{Evidence, FragmentSpec, InputReader, Output, Verdict, WorkloadError};

fn reject(block: u32, committed: Hash32, recomputed: Hash32, reason: &str) -> Verdict {
    Verdict::Reject(Evidence::new(block, committed, recomputed, reason))
}

/// Contrôles 1 et 2 : forme, puis pixels livrés contre milestones. Aucun rendu nécessaire.
fn consistency(header: Header, band: Band, output: &Output) -> Option<Verdict> {
    let zero = Hash32::from_bytes([0; 32]);
    let expected = u64::from(header.width) * u64::from(band.rows()) * 3;
    if output.result.len() as u64 != expected {
        return Some(reject(0, zero, sha256(&output.result), "taille des pixels"));
    }
    if output.milestones.len() != blocks_in(band) as usize {
        return Some(reject(0, zero, zero, "nombre de milestones"));
    }
    let row_bytes = header.width as usize * 3;
    for (index, committed) in output.milestones.iter().enumerate() {
        let (y0, y1) = block_rows(band, index as u32);
        let start = (y0 - band.y_start) as usize * row_bytes;
        let end = (y1 - band.y_start) as usize * row_bytes;
        let delivered = sha256(&output.result[start..end]);
        if delivered != *committed {
            return Some(reject(
                index as u32,
                *committed,
                delivered,
                "pixels livrés différents de l'engagement",
            ));
        }
    }
    None
}

/// Contrôle 3 : recalcul des blocs tirés (hors bornes ou doublons ignorés). Tous les blocs
/// tirés sont rendus d'un seul appel au moteur, puis comparés un à un à leur milestone.
pub(crate) fn check<R: BlockRenderer>(
    workload: &RenderWorkload<R>,
    input: &dyn InputReader,
    spec: &FragmentSpec,
    output: &Output,
    sample: &[u32],
) -> Result<Verdict, WorkloadError> {
    let header = Header::read(input)?;
    let band = Band::from_spec(spec, header.height)?;
    if let Some(verdict) = consistency(header, band, output) {
        return Ok(verdict);
    }
    let blocks = blocks_in(band);
    let mut indices: Vec<u32> = sample.iter().copied().filter(|i| *i < blocks).collect();
    indices.sort_unstable();
    indices.dedup();
    if indices.is_empty() {
        return Ok(Verdict::Approve);
    }
    let scene = Header::scene(input)?;
    let row_bytes = header.width as usize * 3;
    let bands: Vec<(u32, u32)> = indices
        .iter()
        .map(|&index| block_rows(band, index))
        .collect();
    let pixels = workload
        .renderer()
        .render_bands(&scene, header.width, header.height, &bands)?;
    let mut offset = 0usize;
    for (&index, &(y0, y1)) in indices.iter().zip(bands.iter()) {
        let len = (y1 - y0) as usize * row_bytes;
        let block = pixels
            .get(offset..offset + len)
            .ok_or(WorkloadError::Execution("rendu des blocs incomplet".into()))?;
        offset += len;
        let recomputed = sha256(block);
        let committed = output.milestones[index as usize];
        if recomputed != committed {
            return Ok(reject(
                index,
                committed,
                recomputed,
                "bloc recalculé différent",
            ));
        }
    }
    Ok(Verdict::Approve)
}
