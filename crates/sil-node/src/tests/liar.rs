use sil_wl_sha256::Sha256Workload;
use sil_workload_api::{
    FragmentSpec, InputReader, Limits, Output, Registry, Verdict, VerificationPolicy, Workload,
    WorkloadError,
};
use std::sync::Arc;

/// Même identifiant et même vérification que `calcul/sha256`, mais calcule un faux résultat.
pub struct Liar;

impl Workload for Liar {
    fn id(&self) -> &'static str {
        Sha256Workload.id()
    }

    fn limits(&self) -> Limits {
        Sha256Workload.limits()
    }

    fn policy(&self) -> VerificationPolicy {
        Sha256Workload.policy()
    }

    fn split(&self, input: &dyn InputReader, n: u32) -> Result<Vec<FragmentSpec>, WorkloadError> {
        Sha256Workload.split(input, n)
    }

    fn execute(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
    ) -> Result<Output, WorkloadError> {
        let mut output = Sha256Workload.execute(input, spec)?;
        output.result[0] ^= 0xff;
        Ok(output)
    }

    fn block_count(&self, spec: &FragmentSpec) -> Result<u32, WorkloadError> {
        Sha256Workload.block_count(spec)
    }

    fn verify(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
        output: &Output,
        sample: &[u32],
    ) -> Result<Verdict, WorkloadError> {
        Sha256Workload.verify(input, spec, output, sample)
    }
}

pub fn liar_registry() -> Registry {
    Registry::new().with(Arc::new(Liar))
}
