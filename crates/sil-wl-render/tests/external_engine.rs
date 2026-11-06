//! Le moteur externe, lancé dans `sil-sandbox`, rend exactement les octets attendus, et toute
//! panne du moteur (délai, sortie trop grande, sortie courte, programme absent) est une erreur.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_wl_render::{BlockRenderer, ExternalRenderer, RenderWorkload, reference_pixels};
use sil_workload_api::{MemoryInput, Verdict, Workload, WorkloadError};
use std::{path::PathBuf, time::Duration};

const ENGINE: &str = env!("CARGO_BIN_EXE_sil-render-reference");

fn engine(wall: Duration) -> ExternalRenderer {
    ExternalRenderer::new(PathBuf::from(ENGINE), std::env::temp_dir(), wall)
}

fn scene() -> Vec<u8> {
    (0..64u8).map(|i| i.wrapping_mul(29) ^ 0x5A).collect()
}

/// En-tête (largeur, hauteur en u32 little-endian) puis la scène.
fn image(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(scene());
    bytes
}

#[test]
fn the_engine_renders_exactly_the_reference_pixels() {
    let rendered = engine(Duration::from_secs(30))
        .render(&scene(), 16, 24, 8, 16)
        .expect("rendu");
    assert_eq!(rendered, reference_pixels(&scene(), 16, 8, 16));
    assert_eq!(rendered.len(), 16 * 8 * 3);
}

#[test]
fn a_render_workload_runs_its_fragments_through_the_engine() {
    let bytes = image(16, 24);
    let input = MemoryInput(&bytes);
    let workload = RenderWorkload::new(engine(Duration::from_secs(30)));
    let specs = workload.split(&input, 2).expect("découpe");
    assert_eq!(specs.len(), 2);
    let output = workload.execute(&input, &specs[0]).expect("exécution");
    // Première bande : 2 blocs de 8 lignes, un hash par bloc, 16 pixels de large.
    assert_eq!(output.result.len(), 16 * 16 * 3);
    assert_eq!(output.milestones.len(), 2);
    let verdict = workload
        .verify(&input, &specs[0], &output, &[0])
        .expect("vérification");
    assert_eq!(verdict, Verdict::Approve, "un rendu honnête est approuvé");
}

#[test]
fn a_slow_engine_is_killed_at_the_deadline() {
    let renderer = engine(Duration::from_millis(500)).with_extra_args(vec!["slow".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("délai dépassé");
    assert!(matches!(error, WorkloadError::Execution(text) if text.contains("délai")));
}

#[test]
fn an_engine_with_runaway_output_is_cut_off() {
    let renderer = engine(Duration::from_secs(30)).with_extra_args(vec!["flood".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("sortie trop grande");
    assert!(matches!(error, WorkloadError::OverLimit(_)), "{error:?}");
}

#[test]
fn an_engine_that_returns_too_few_bytes_is_refused() {
    let renderer = engine(Duration::from_secs(30)).with_extra_args(vec!["short".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("octets manquants");
    assert!(matches!(error, WorkloadError::Execution(text) if text.contains("octets rendus")));
}

#[test]
fn a_missing_engine_is_an_error_not_a_panic() {
    let renderer = ExternalRenderer::new(
        PathBuf::from("does-not-exist-sil-engine"),
        std::env::temp_dir(),
        Duration::from_secs(5),
    );
    assert!(renderer.render(&scene(), 16, 24, 0, 8).is_err());
}
