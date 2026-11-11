//! Workload `embedding/e5-small` : chaque ligne de l'entrée est un texte, chaque texte reçoit son
//! vecteur. Un fragment est une plage de lignes. Vérification par tolérance (mode
//! `SamplingTolerance`) : un vérificateur recalcule quelques textes tirés et compare par cosinus.

use crate::{DIM, Engine};
use sha2::{Digest, Sha256};
use sil_crypto::sha256;
use sil_sampling::cosine;
use sil_types::Hash32;
use sil_workload_api::{
    FragmentSpec, InputReader, Limits, Output, ToleranceJudgement, Verdict, VerificationPolicy,
    Workload, WorkloadError, limits_of, policy_of,
};
use sil_workload_spec::EMBEDDING_E5_SMALL;
use std::sync::Mutex;

/// Nombre maximal de fragments d'un job.
const MAX_FRAGMENTS: u32 = 1024;
/// Taille d'un vecteur en octets (f32 little-endian).
const VECTOR_BYTES: usize = DIM * 4;

/// Plage de lignes d'un fragment : première ligne et nombre de lignes, en octets canoniques.
fn encode_range(first: u32, count: u32) -> Vec<u8> {
    let mut params = Vec::with_capacity(8);
    params.extend_from_slice(&first.to_le_bytes());
    params.extend_from_slice(&count.to_le_bytes());
    params
}

fn decode_range(params: &[u8]) -> Result<(usize, usize), WorkloadError> {
    let bytes: [u8; 8] = params
        .try_into()
        .map_err(|_| WorkloadError::BadSpec("plage de lignes".into()))?;
    let first = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    Ok((first as usize, count as usize))
}

/// Les lignes non vides de l'entrée, dans l'ordre (une ligne = un texte).
fn lines_of(input: &dyn InputReader) -> Result<Vec<String>, WorkloadError> {
    let bytes = input.read(0, input.len() as usize)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| WorkloadError::BadInput("UTF-8 attendu".into()))?;
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect())
}

/// Encode des vecteurs en octets (f32 little-endian, dans l'ordre).
pub fn encode_vectors(vectors: &[Vec<f32>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vectors.len() * VECTOR_BYTES);
    for vector in vectors {
        for value in vector {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

/// Le vecteur numéro `index` d'un résultat encodé, s'il existe.
pub fn decode_vector(result: &[u8], index: usize) -> Option<Vec<f32>> {
    let start = index.checked_mul(VECTOR_BYTES)?;
    let bytes = result.get(start..start + VECTOR_BYTES)?;
    Some(
        bytes
            .chunks(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

/// Hash d'un vecteur : un bloc vérifiable par texte.
fn milestone_of(vector: &[u8]) -> Hash32 {
    sha256(vector)
}

/// Le workload d'embedding, avec son moteur (chargé une fois par le nœud).
pub struct EmbedWorkload {
    engine: Mutex<Engine>,
}

impl EmbedWorkload {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Mutex::new(engine),
        }
    }
}

impl Workload for EmbedWorkload {
    fn id(&self) -> &'static str {
        EMBEDDING_E5_SMALL.id
    }

    fn limits(&self) -> Limits {
        limits_of(&EMBEDDING_E5_SMALL)
    }

    fn policy(&self) -> VerificationPolicy {
        policy_of(&EMBEDDING_E5_SMALL)
    }

    fn split(
        &self,
        input: &dyn InputReader,
        fragments: u32,
    ) -> Result<Vec<FragmentSpec>, WorkloadError> {
        if fragments == 0 || fragments > MAX_FRAGMENTS {
            return Err(WorkloadError::BadInput("nombre de fragments".into()));
        }
        let lines = lines_of(input)?.len() as u32;
        let count = fragments.min(lines.max(1));
        let (base, extra) = (lines / count, lines % count);
        let mut first = 0u32;
        Ok((0..count)
            .map(|index| {
                let size = base + u32::from(index < extra);
                let spec = FragmentSpec {
                    index,
                    params: encode_range(first, size),
                };
                first += size;
                spec
            })
            .collect())
    }

    fn execute(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
    ) -> Result<Output, WorkloadError> {
        let (first, count) = decode_range(&spec.params)?;
        let lines = lines_of(input)?;
        let texts = lines
            .get(first..first + count)
            .ok_or_else(|| WorkloadError::BadSpec("plage hors de l'entrée".into()))?;
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| WorkloadError::Execution("moteur verrouillé".into()))?;
        let mut vectors = Vec::with_capacity(texts.len());
        for text in texts {
            vectors.push(
                engine
                    .embed(text)
                    .map_err(|error| WorkloadError::Execution(error.to_string()))?,
            );
        }
        let result = encode_vectors(&vectors);
        let milestones = result.chunks(VECTOR_BYTES).map(milestone_of).collect();
        Ok(Output { result, milestones })
    }

    fn block_count(&self, spec: &FragmentSpec) -> Result<u32, WorkloadError> {
        let (_, count) = decode_range(&spec.params)?;
        Ok(count as u32)
    }

    fn verify(
        &self,
        _input: &dyn InputReader,
        _spec: &FragmentSpec,
        _output: &Output,
        _sample: &[u32],
    ) -> Result<Verdict, WorkloadError> {
        // Pas de comparaison à l'octet pour ce workload : voir verify_tolerance.
        Err(WorkloadError::Unsupported)
    }

    fn verify_tolerance(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
        output: &Output,
        sample: &[u32],
    ) -> Result<ToleranceJudgement, WorkloadError> {
        let (first, count) = decode_range(&spec.params)?;
        let lines = lines_of(input)?;
        let texts = lines
            .get(first..first + count)
            .ok_or_else(|| WorkloadError::BadSpec("plage hors de l'entrée".into()))?;
        // Rien à comparer : pas d'approbation par défaut.
        if sample.is_empty() {
            return Ok(ToleranceJudgement {
                min_cos_micro: i32::MIN,
                sample_root: Hash32::from_bytes([0; 32]),
            });
        }
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| WorkloadError::Execution("moteur verrouillé".into()))?;
        let mut recomputed = Vec::with_capacity(sample.len());
        let mut received = Vec::with_capacity(sample.len());
        let mut root = Sha256::new();
        for &index in sample {
            let index = index as usize;
            let text = texts
                .get(index)
                .ok_or_else(|| WorkloadError::BadSpec("bloc tiré hors du fragment".into()))?;
            let mine = engine
                .embed(text)
                .map_err(|error| WorkloadError::Execution(error.to_string()))?;
            let theirs = decode_vector(&output.result, index)
                .ok_or_else(|| WorkloadError::BadInput("vecteur absent du résultat".into()))?;
            root.update((index as u32).to_le_bytes());
            root.update(encode_vectors(std::slice::from_ref(&mine)));
            recomputed.push(mine);
            received.push(theirs);
        }
        let sample_root = Hash32::from_bytes(root.finalize().into());
        // Un cosinus absent ou non fini (débordement) est un rejet : f64::min ignorerait un NaN.
        let mut min = f64::INFINITY;
        for (mine, theirs) in recomputed.iter().zip(&received) {
            match cosine(theirs, mine) {
                Some(value) if value.is_finite() => min = min.min(value),
                _ => {
                    return Ok(ToleranceJudgement {
                        min_cos_micro: i32::MIN,
                        sample_root,
                    });
                }
            }
        }
        Ok(ToleranceJudgement {
            min_cos_micro: cos_to_micro(min),
            sample_root,
        })
    }
}

/// Cosinus en millionièmes, borné à l'intervalle de i32 (un cosinus NaN ou infini est un rejet).
fn cos_to_micro(value: f64) -> i32 {
    let scaled = (value * 1_000_000.0).round();
    if !scaled.is_finite() {
        return i32::MIN;
    }
    scaled.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_round_trips() {
        assert_eq!(decode_range(&encode_range(7, 3)).unwrap(), (7, 3));
        assert!(decode_range(&[1, 2, 3]).is_err());
    }

    #[test]
    fn vectors_round_trip_through_the_result_bytes() {
        let vectors = vec![vec![0.5f32; DIM], vec![-1.25f32; DIM]];
        let bytes = encode_vectors(&vectors);
        assert_eq!(bytes.len(), 2 * VECTOR_BYTES);
        assert_eq!(decode_vector(&bytes, 0).unwrap(), vectors[0]);
        assert_eq!(decode_vector(&bytes, 1).unwrap(), vectors[1]);
        assert!(decode_vector(&bytes, 2).is_none());
    }

    #[test]
    fn a_nan_cosine_is_a_rejection() {
        assert_eq!(cos_to_micro(f64::NAN), i32::MIN);
        assert_eq!(cos_to_micro(0.999_5), 999_500);
        assert_eq!(cos_to_micro(2.0), 2_000_000);
        // Débordement : rejet, pas de valeur saturée qui passerait le seuil.
        assert_eq!(cos_to_micro(f64::MAX), i32::MIN);
    }
}
