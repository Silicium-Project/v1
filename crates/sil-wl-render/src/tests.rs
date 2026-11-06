use crate::{BlockRenderer, ROWS_PER_BLOCK, RenderWorkload};
use sil_crypto::{MasterSeed, Purpose, sha256};
use sil_sampling::{SamplingSeed, detection_probability, sample_indices, samples_needed};
use sil_types::{Hash32, JobId, TaskRef};
use sil_workload_api::{FragmentSpec, MemoryInput, Output, Verdict, Workload, WorkloadError};

/// Moteur factice déterministe : le pixel dépend de (x, y, scène).
struct Fake;

impl BlockRenderer for Fake {
    fn render(
        &self,
        scene: &[u8],
        width: u32,
        _height: u32,
        y0: u32,
        y1: u32,
    ) -> Result<Vec<u8>, WorkloadError> {
        let tint = scene.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        let mut out = Vec::new();
        for y in y0..y1 {
            for x in 0..width {
                for channel in 0..3u32 {
                    out.push((x * 31 + y * 17 + channel * 5) as u8 ^ tint);
                }
            }
        }
        Ok(out)
    }
}

fn input_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = width.to_le_bytes().to_vec();
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(b"scene-data");
    bytes
}

fn workload() -> RenderWorkload<Fake> {
    RenderWorkload::new(Fake)
}

#[test]
fn fragments_are_block_aligned_and_cover_every_row() {
    let bytes = input_bytes(4, 803);
    let input = MemoryInput(&bytes);
    let specs = workload().split(&input, 5).unwrap();
    assert_eq!(specs.len(), 5);
    let blocks: u32 = specs
        .iter()
        .map(|s| workload().block_count(s).unwrap())
        .sum();
    assert_eq!(blocks, 803u32.div_ceil(ROWS_PER_BLOCK));
    let outputs: Vec<Output> = specs
        .iter()
        .map(|s| workload().execute(&input, s).unwrap())
        .collect();
    let image = workload().combine(&outputs).unwrap();
    assert_eq!(
        image.len(),
        4 * 803 * 3,
        "toutes les lignes, sans trou ni recouvrement"
    );
    let direct = Fake.render(b"scene-data", 4, 803, 0, 803).unwrap();
    assert_eq!(
        image, direct,
        "l'assemblage égale un rendu d'un seul tenant"
    );
}

#[test]
fn invalid_headers_and_bands_are_refused() {
    assert!(workload().split(&MemoryInput(&[1, 2]), 2).is_err());
    let zero = input_bytes(0, 10);
    assert!(workload().split(&MemoryInput(&zero), 2).is_err());
    let huge = input_bytes(1_000_000, 10);
    assert!(workload().split(&MemoryInput(&huge), 2).is_err());
    let bytes = input_bytes(4, 16);
    let garbled = FragmentSpec {
        index: 0,
        params: vec![9, 9],
    };
    assert!(workload().execute(&MemoryInput(&bytes), &garbled).is_err());
}

fn honest(rows: u32) -> (Vec<u8>, FragmentSpec, Output) {
    let bytes = input_bytes(4, rows);
    let input = MemoryInput(&bytes);
    let spec = workload().split(&input, 1).unwrap().remove(0);
    let output = workload().execute(&input, &spec).unwrap();
    (bytes, spec, output)
}

#[test]
fn an_honest_result_is_approved_with_or_without_a_sample() {
    let (bytes, spec, output) = honest(80);
    let input = MemoryInput(&bytes);
    let all: Vec<u32> = (0..10).collect();
    assert_eq!(
        workload().verify(&input, &spec, &output, &[]).unwrap(),
        Verdict::Approve
    );
    assert_eq!(
        workload().verify(&input, &spec, &output, &all).unwrap(),
        Verdict::Approve
    );
    assert_eq!(
        workload().verify(&input, &spec, &output, &[999]).unwrap(),
        Verdict::Approve
    );
}

#[test]
fn pixels_that_differ_from_the_committed_hashes_are_caught_without_any_sample() {
    let (bytes, spec, mut output) = honest(80);
    output.result[5] ^= 0xff; // bloc 0 modifié, milestones inchangés
    match workload()
        .verify(&MemoryInput(&bytes), &spec, &output, &[])
        .unwrap()
    {
        Verdict::Reject(evidence) => assert_eq!(evidence.block, 0),
        Verdict::Approve => panic!("pixels falsifiés approuvés"),
    }
}

#[test]
fn malformed_outputs_are_rejected() {
    let (bytes, spec, output) = honest(80);
    let input = MemoryInput(&bytes);
    let mut short = output.clone();
    short.result.pop();
    assert!(matches!(
        workload().verify(&input, &spec, &short, &[]).unwrap(),
        Verdict::Reject(_)
    ));
    let mut missing = output;
    missing.milestones.pop();
    assert!(matches!(
        workload().verify(&input, &spec, &missing, &[]).unwrap(),
        Verdict::Reject(_)
    ));
}

/// Un worker paresseux : remplace des blocs par du noir et engage les hash du noir
/// (cohérent en interne ; seul un recalcul le démasque).
fn lazy(output: &Output, bad: &[u32], width: u32) -> Output {
    let mut cheat = output.clone();
    let row_bytes = width as usize * 3;
    for &block in bad {
        let start = block as usize * ROWS_PER_BLOCK as usize * row_bytes;
        let end =
            ((block as usize + 1) * ROWS_PER_BLOCK as usize * row_bytes).min(cheat.result.len());
        cheat.result[start..end].fill(0);
        cheat.milestones[block as usize] = sha256(&cheat.result[start..end]);
    }
    cheat
}

fn draw(n: u32, total: u32, k: u32) -> Vec<u32> {
    let id = MasterSeed::from_bytes([(n % 250) as u8 + 1; 32]).derive(Purpose::Ledger);
    let task = TaskRef::new(JobId::from_bytes([1; 32]), 0);
    let result = Hash32::from_bytes([(n / 250) as u8; 32]);
    let (seed, _) = SamplingSeed::draw(&id, &task, 1, &result);
    sample_indices(&seed, total, k)
}

#[test]
fn a_lazy_worker_is_caught_at_the_targeted_rate_by_sampled_recomputation() {
    let (bytes, spec, output) = honest(800); // 100 blocs
    let input = MemoryInput(&bytes);
    let cheat = lazy(&output, &(0..20).collect::<Vec<_>>(), 4);
    assert!(
        workload().verify(&input, &spec, &cheat, &[]).unwrap() == Verdict::Approve,
        "sans recalcul, la triche cohérente passe : c'est tout l'intérêt de l'échantillon"
    );
    let k = samples_needed(100, 0.2, 0.99);
    let caught = (0..300u32)
        .filter(|n| {
            let sample = draw(*n, 100, k);
            matches!(
                workload().verify(&input, &spec, &cheat, &sample).unwrap(),
                Verdict::Reject(_)
            )
        })
        .count();
    assert!(
        caught >= 290,
        "{caught}/300 détectés avec {k} blocs recalculés"
    );
}

#[test]
fn a_single_forged_block_is_rarely_caught_and_this_is_documented() {
    let k = samples_needed(100, 0.2, 0.99);
    let chance = detection_probability(100, 1, k);
    assert!(chance < 0.25, "{chance}");
    // Le mécanisme n'est donc pas une preuve de calcul : voir docs/technical/protocol/verification.md.
}
