use crate::range::Range;
use sha2::{Digest, Sha256};
use sil_types::Hash32;
use sil_workload_api::{
    Evidence, FragmentSpec, InputReader, Limits, Output, Verdict, VerificationPolicy, Workload,
    WorkloadError, limits_of, policy_of,
};
use sil_workload_spec::CALCUL_SHA256;

/// Lecture par morceaux de 1 Mio : mémoire constante quelle que soit la plage.
const READ_STEP: u64 = 1024 * 1024;
const MAX_FRAGMENTS: u32 = 1024;

#[derive(Debug, Clone, Copy, Default)]
pub struct Sha256Workload;

fn digest_of(input: &dyn InputReader, range: Range) -> Result<Hash32, WorkloadError> {
    let mut hasher = Sha256::new();
    let mut position = range.start;
    while position < range.end {
        let step = (range.end - position).min(READ_STEP);
        hasher.update(input.read(position, step as usize)?);
        position += step;
    }
    Ok(Hash32::from_bytes(hasher.finalize().into()))
}

impl Workload for Sha256Workload {
    fn id(&self) -> &'static str {
        CALCUL_SHA256.id
    }

    fn limits(&self) -> Limits {
        limits_of(&CALCUL_SHA256)
    }

    fn policy(&self) -> VerificationPolicy {
        policy_of(&CALCUL_SHA256)
    }

    fn split(
        &self,
        input: &dyn InputReader,
        fragments: u32,
    ) -> Result<Vec<FragmentSpec>, WorkloadError> {
        if fragments == 0 || fragments > MAX_FRAGMENTS {
            return Err(WorkloadError::BadInput("nombre de fragments".into()));
        }
        let len = input.len();
        let count = u64::from(fragments).min(len.max(1));
        let (base, extra) = (len / count, len % count);
        let mut start = 0;
        (0..count)
            .map(|i| {
                let end = start + base + u64::from(i < extra);
                let spec = Range { start, end }.to_spec(i as u32);
                start = end;
                spec
            })
            .collect()
    }

    fn execute(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
    ) -> Result<Output, WorkloadError> {
        let range = Range::from_spec(spec, input.len())?;
        let digest = digest_of(input, range)?;
        Ok(Output {
            result: digest.as_bytes().to_vec(),
            milestones: Vec::new(),
        })
    }

    fn block_count(&self, _spec: &FragmentSpec) -> Result<u32, WorkloadError> {
        Ok(1)
    }

    fn verify(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
        output: &Output,
        _sample: &[u32],
    ) -> Result<Verdict, WorkloadError> {
        let range = Range::from_spec(spec, input.len())?;
        let expected = digest_of(input, range)?;
        let claimed: Option<[u8; 32]> = output.result.as_slice().try_into().ok();
        let claimed_hash = Hash32::from_bytes(claimed.unwrap_or([0; 32]));
        if claimed == Some(*expected.as_bytes()) {
            return Ok(Verdict::Approve);
        }
        let reason = if claimed.is_none() {
            "résultat de taille inattendue"
        } else {
            "hash différent"
        };
        Ok(Verdict::Reject(Evidence::new(
            0,
            claimed_hash,
            expected,
            reason,
        )))
    }

    fn combine(&self, outputs: &[Output]) -> Result<Vec<u8>, WorkloadError> {
        let mut hasher = Sha256::new();
        for output in outputs {
            hasher.update(&output.result);
        }
        Ok(hasher.finalize().to_vec())
    }
}
