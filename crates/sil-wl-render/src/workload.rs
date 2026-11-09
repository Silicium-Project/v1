use crate::{
    band::{Band, Header},
    renderer::BlockRenderer,
    verify,
};
use sil_crypto::sha256;
use sil_workload_api::{
    FragmentSpec, InputReader, Limits, Output, Verdict, VerificationPolicy, Workload,
    WorkloadError, limits_of, policy_of,
};
use sil_workload_spec::RENDU_RAYTRACER;

/// Lignes par bloc vérifiable (descripteur `rendu/raytracer`).
pub const ROWS_PER_BLOCK: u32 = RENDU_RAYTRACER.block_rows;
const MAX_FRAGMENTS: u32 = 1024;
const MAX_OUTPUT_BYTES: u64 = RENDU_RAYTRACER.limits.output_bytes;

pub struct RenderWorkload<R> {
    renderer: R,
}

impl<R: BlockRenderer> RenderWorkload<R> {
    pub fn new(renderer: R) -> Self {
        Self { renderer }
    }

    pub(crate) fn renderer(&self) -> &R {
        &self.renderer
    }
}

/// Plage de lignes du bloc `index` dans la bande.
pub(crate) fn block_rows(band: Band, index: u32) -> (u32, u32) {
    let y0 = band.y_start + index * ROWS_PER_BLOCK;
    (y0, (y0 + ROWS_PER_BLOCK).min(band.y_end))
}

pub(crate) fn blocks_in(band: Band) -> u32 {
    band.rows().div_ceil(ROWS_PER_BLOCK)
}

impl<R: BlockRenderer> Workload for RenderWorkload<R> {
    fn id(&self) -> &'static str {
        RENDU_RAYTRACER.id
    }

    fn limits(&self) -> Limits {
        limits_of(&RENDU_RAYTRACER)
    }

    fn policy(&self) -> VerificationPolicy {
        policy_of(&RENDU_RAYTRACER)
    }

    fn split(
        &self,
        input: &dyn InputReader,
        fragments: u32,
    ) -> Result<Vec<FragmentSpec>, WorkloadError> {
        if fragments == 0 || fragments > MAX_FRAGMENTS {
            return Err(WorkloadError::BadInput("nombre de fragments".into()));
        }
        let header = Header::read(input)?;
        let blocks = header.height.div_ceil(ROWS_PER_BLOCK);
        let count = fragments.min(blocks);
        let (base, extra) = (blocks / count, blocks % count);
        let mut first = 0;
        (0..count)
            .map(|i| {
                let last = first + base + u32::from(i < extra);
                let band = Band {
                    y_start: first * ROWS_PER_BLOCK,
                    y_end: (last * ROWS_PER_BLOCK).min(header.height),
                };
                first = last;
                band.to_spec(i)
            })
            .collect()
    }

    fn execute(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
    ) -> Result<Output, WorkloadError> {
        let header = Header::read(input)?;
        let band = Band::from_spec(spec, header.height)?;
        let bytes = u64::from(header.width) * u64::from(band.rows()) * 3;
        if bytes > MAX_OUTPUT_BYTES {
            return Err(WorkloadError::OverLimit("taille du résultat"));
        }
        let scene = Header::scene(input)?;
        let mut result = Vec::with_capacity(bytes as usize);
        let mut milestones = Vec::new();
        for index in 0..blocks_in(band) {
            let (y0, y1) = block_rows(band, index);
            let block = self
                .renderer
                .render(&scene, header.width, header.height, y0, y1)?;
            milestones.push(sha256(&block));
            result.extend_from_slice(&block);
        }
        Ok(Output { result, milestones })
    }

    fn check_shape(
        &self,
        input: &dyn InputReader,
        width: u32,
        height: u32,
    ) -> Result<(), WorkloadError> {
        let header = Header::read(input)?;
        if (header.width, header.height) != (width, height) {
            return Err(WorkloadError::BadInput(
                "dimensions de l'en-tête différentes de la spec".into(),
            ));
        }
        Ok(())
    }

    fn block_count(&self, spec: &FragmentSpec) -> Result<u32, WorkloadError> {
        // Sans l'entrée, la bande seule donne le nombre de blocs.
        let band: Band = sil_codec::decode(&spec.params, 32)
            .map_err(|e| WorkloadError::BadSpec(e.to_string()))?;
        Ok(blocks_in(band))
    }

    fn verify(
        &self,
        input: &dyn InputReader,
        spec: &FragmentSpec,
        output: &Output,
        sample: &[u32],
    ) -> Result<Verdict, WorkloadError> {
        verify::check(self, input, spec, output, sample)
    }

    fn combine(&self, outputs: &[Output]) -> Result<Vec<u8>, WorkloadError> {
        Ok(outputs
            .iter()
            .flat_map(|o| o.result.iter().copied())
            .collect())
    }
}
