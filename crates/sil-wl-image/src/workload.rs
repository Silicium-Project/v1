use crate::codec::process;
use sil_batch::{Batch, Header, Kind, frame, parse, parse_range, range_params, unframe};
use sil_crypto::sha256;
use sil_types::Hash32;
use sil_workload_api::{
    Evidence, FragmentSpec, InputReader, Limits, Output, Verdict, VerificationPolicy, Workload,
    WorkloadError, limits_of, policy_of,
};
use sil_workload_spec::IMAGE_BATCH;

/// Nombre maximal de fragments d'un job (la spec du descripteur).
const MAX_FRAGMENTS: u32 = 16;

/// Workload `image/batch` : redimensionne et réencode chaque image d'un lot SILBATCH. Un fragment
/// est une plage d'images. Chaque image est un bloc vérifiable : le résultat engage le hash de
/// chaque image encodée, et un vérificateur refait un échantillon d'images (politique `Sampling`).
#[derive(Debug, Clone, Copy, Default)]
pub struct ImageBatchWorkload;

/// Traite les images d'une plage sur plusieurs threads. Les résultats restent dans l'ordre de la
/// plage : le résultat ne dépend pas du nombre de threads.
fn process_all(records: &[&[u8]], header: Header) -> Result<Vec<Vec<u8>>, WorkloadError> {
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(records.len().max(1));
    if threads <= 1 {
        return records
            .iter()
            .map(|record| process(record, header))
            .collect();
    }
    let chunk = records.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = records
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|record| process(record, header))
                        .collect::<Result<Vec<_>, _>>()
                })
            })
            .collect();
        let mut out = Vec::with_capacity(records.len());
        for handle in handles {
            let part = handle.join().map_err(|_| {
                WorkloadError::Execution("traitement des images interrompu".into())
            })??;
            out.extend(part);
        }
        Ok(out)
    })
}

/// Lit le lot entier (les entrées de job sont bornées par le descripteur).
fn read_batch(input: &dyn InputReader) -> Result<Vec<u8>, WorkloadError> {
    let len = usize::try_from(input.len()).map_err(|_| WorkloadError::OverLimit("entrée"))?;
    input.read(0, len)
}

/// Plage `[start, end)` d'un fragment, contrôlée contre le lot.
fn range_of(spec: &FragmentSpec, count: u32) -> Result<(u32, u32), WorkloadError> {
    parse_range(&spec.params, count).map_err(|e| WorkloadError::BadSpec(e.to_string()))
}

fn reject(block: u32, committed: Hash32, recomputed: Hash32, reason: &str) -> Verdict {
    Verdict::Reject(Evidence::new(block, committed, recomputed, reason))
}

impl Workload for ImageBatchWorkload {
    fn id(&self) -> &'static str {
        IMAGE_BATCH.id
    }

    fn limits(&self) -> Limits {
        limits_of(&IMAGE_BATCH)
    }

    fn policy(&self) -> VerificationPolicy {
        policy_of(&IMAGE_BATCH)
    }

    fn split(
        &self,
        input: &dyn InputReader,
        fragments: u32,
    ) -> Result<Vec<FragmentSpec>, WorkloadError> {
        if fragments == 0 || fragments > MAX_FRAGMENTS {
            return Err(WorkloadError::BadInput("nombre de fragments".into()));
        }
        let bytes = read_batch(input)?;
        let batch = parse(&bytes).map_err(|e| WorkloadError::BadInput(e.to_string()))?;
        if batch.header.kind != Kind::Image {
            return Err(WorkloadError::BadInput(
                "le lot n'est pas un lot d'images".into(),
            ));
        }
        // Un fragment par demande, même s'il y a moins d'images : les fragments en trop ont une
        // plage vide (résultat vide), sinon le job ne trouverait pas le fragment qu'il attend.
        let count = batch.header.count;
        let (base, extra) = (count / fragments, count % fragments);
        let mut start = 0u32;
        (0..fragments)
            .map(|index| {
                let end = start + base + u32::from(index < extra);
                let spec = FragmentSpec {
                    index,
                    params: range_params(start, end),
                };
                start = end;
                Ok(spec)
            })
            .collect()
    }

    fn execute(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
    ) -> Result<Output, WorkloadError> {
        let bytes = read_batch(input)?;
        let batch: Batch<'_> = parse(&bytes).map_err(|e| WorkloadError::BadInput(e.to_string()))?;
        let (start, end) = range_of(spec, batch.header.count)?;
        let encoded = process_all(&batch.records[start as usize..end as usize], batch.header)?;
        let milestones = encoded.iter().map(|image| sha256(image)).collect();
        let result = frame(&encoded).map_err(|e| WorkloadError::Execution(e.to_string()))?;
        if result.len() as u64 > IMAGE_BATCH.limits.output_bytes {
            return Err(WorkloadError::OverLimit("résultat du fragment"));
        }
        Ok(Output { result, milestones })
    }

    /// Un bloc par image de la plage.
    fn block_count(&self, spec: &FragmentSpec) -> Result<u32, WorkloadError> {
        let (start, end) = range_of(spec, u32::MAX)?;
        Ok(end - start)
    }

    fn verify(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
        output: &Output,
        sample: &[u32],
    ) -> Result<Verdict, WorkloadError> {
        let bytes = read_batch(input)?;
        let batch = parse(&bytes).map_err(|e| WorkloadError::BadInput(e.to_string()))?;
        let (start, end) = range_of(spec, batch.header.count)?;
        let images = (end - start) as usize;
        let zero = Hash32::from_bytes([0; 32]);

        // Contrôle de forme et d'engagement, sans rendu : chaque image livrée doit correspondre à
        // son milestone.
        let delivered = match unframe(&output.result) {
            Ok(parts) if parts.len() == images && output.milestones.len() == images => parts,
            _ => {
                return Ok(reject(
                    0,
                    zero,
                    sha256(&output.result),
                    "cadre de résultat invalide",
                ));
            }
        };
        for (index, part) in delivered.iter().enumerate() {
            let committed = output.milestones[index];
            let hash = sha256(part);
            if hash != committed {
                return Ok(reject(
                    index as u32,
                    committed,
                    hash,
                    "image livrée différente de l'engagement",
                ));
            }
        }

        // Contrôle par échantillon : chaque image tirée est refaite.
        let mut indices: Vec<u32> = sample
            .iter()
            .copied()
            .filter(|index| (*index as usize) < images)
            .collect();
        indices.sort_unstable();
        indices.dedup();
        for index in indices {
            let record = batch.records[(start + index) as usize];
            let committed = output.milestones[index as usize];
            match process(record, batch.header) {
                Ok(encoded) => {
                    let recomputed = sha256(&encoded);
                    if recomputed != committed {
                        return Ok(reject(
                            index,
                            committed,
                            recomputed,
                            "image recalculée différente",
                        ));
                    }
                }
                // Une image que le moteur ne sait pas traiter ne peut pas avoir été traitée honnêtement.
                Err(_) => {
                    return Ok(reject(index, committed, zero, "image non traitable"));
                }
            }
        }
        Ok(Verdict::Approve)
    }

    fn combine(&self, outputs: &[Output]) -> Result<Vec<u8>, WorkloadError> {
        // Chaque résultat est déjà un cadre : la concaténation est un cadre valide.
        Ok(outputs
            .iter()
            .flat_map(|o| o.result.iter().copied())
            .collect())
    }
}
