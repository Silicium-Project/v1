use sil_wl_sha256::Sha256Workload;
use sil_workload_api::{
    FragmentSpec, InputReader, Limits, Output, Registry, Verdict, VerificationPolicy, Workload,
    WorkloadError,
};
use std::{sync::Arc, time::Duration};

/// Même calcul que `calcul/sha256`, mais qui prend du temps (plus long que le bail du test).
pub struct Slow(pub Duration);

impl Workload for Slow {
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
        std::thread::sleep(self.0);
        Sha256Workload.execute(input, spec)
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

pub fn slow_registry(duration: Duration) -> Registry {
    Registry::new().with(Arc::new(Slow(duration)))
}
