/// Résultat d'un verdict par tolérance : le plus petit cosinus observé et la racine qui engage les
/// vecteurs recalculés (elle sert de preuve dans le message signé).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToleranceJudgement {
    pub min_cos_micro: i32,
    pub sample_root: sil_types::Hash32,
}

use crate::{
    FragmentSpec, InputReader, Limits, Output, Verdict, VerificationPolicy, WorkloadError,
};

/// Un type de calcul. Les méthodes sont synchrones et gourmandes en CPU : le nœud les
/// appelle depuis un thread dédié (jamais dans la boucle réseau).
pub trait Workload: Send + Sync {
    /// Identifiant stable (`a-z0-9/._-`), celui de `JobSpec::workload`.
    fn id(&self) -> &'static str;

    fn limits(&self) -> Limits;

    fn policy(&self) -> VerificationPolicy;

    /// Découpe l'entrée en au plus `fragments` fragments.
    fn split(
        &self,
        input: &dyn InputReader,
        fragments: u32,
    ) -> Result<Vec<FragmentSpec>, WorkloadError>;

    /// Vérifie que l'entrée a la forme que la spec déclare (`width`, `height`, pour un workload
    /// avec image). Par défaut, rien à vérifier.
    fn check_shape(
        &self,
        _input: &dyn InputReader,
        _width: u32,
        _height: u32,
    ) -> Result<(), WorkloadError> {
        Ok(())
    }

    /// Calcule un fragment.
    fn execute(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
    ) -> Result<Output, WorkloadError>;

    /// Nombre de blocs vérifiables du fragment (base de l'échantillonnage).
    fn block_count(&self, spec: &FragmentSpec) -> Result<u32, WorkloadError>;

    /// Vérifie le résultat d'un autre nœud. `sample` : blocs à recalculer (indices tirés par
    /// `sil-sampling`) ; ignoré par la politique `Recompute`.
    fn verify(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
        output: &Output,
        sample: &[u32],
    ) -> Result<Verdict, WorkloadError>;

    /// Verdict par tolérance (mode `SamplingTolerance`, embedding) : recalcule les blocs tirés et
    /// rend le cosinus minimal observé, en millionièmes, avec une racine qui engage les vecteurs
    /// recalculés. Par défaut : non supporté.
    fn verify_tolerance(
        &self,
        _input: &dyn InputReader,
        _spec: &FragmentSpec,
        _output: &Output,
        _sample: &[u32],
    ) -> Result<ToleranceJudgement, WorkloadError> {
        Err(WorkloadError::Unsupported)
    }

    /// Assemble les résultats des fragments (dans l'ordre) en résultat du job.
    fn combine(&self, _outputs: &[Output]) -> Result<Vec<u8>, WorkloadError> {
        Err(WorkloadError::Unsupported)
    }
}
