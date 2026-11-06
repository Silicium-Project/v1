//! Le moteur WebAssembly rend exactement les octets attendus, et chaque panne ou tentative
//! interdite (délai, sortie trop grande, réseau, écriture, mémoire) est refusée.
//!
//! Le moteur de référence doit être compilé avant :
//! `cargo build -p sil-wl-render --bin sil-render-reference --target wasm32-wasip1`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_render_wasm::WasmRenderer;
use sil_wl_render::{BlockRenderer, RenderWorkload, reference_pixels};
use sil_workload_api::{MemoryInput, Verdict, Workload, WorkloadError};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

const MIB: usize = 1024 * 1024;

fn engine_path() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-wasip1/debug/sil-render-reference.wasm");
    assert!(
        path.exists(),
        "moteur absent : cargo build -p sil-wl-render --bin sil-render-reference --target wasm32-wasip1"
    );
    path
}

fn engine(memory: usize, wall: Duration) -> WasmRenderer {
    WasmRenderer::new(&engine_path(), memory, wall).expect("moteur chargé")
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
fn the_wasm_engine_renders_exactly_the_reference_pixels() {
    let rendered = engine(256 * MIB, Duration::from_secs(30))
        .render(&scene(), 16, 24, 8, 16)
        .expect("rendu");
    assert_eq!(rendered, reference_pixels(&scene(), 16, 8, 16));
}

#[test]
fn a_render_workload_runs_its_fragments_on_the_wasm_engine() {
    let bytes = image(16, 24);
    let input = MemoryInput(&bytes);
    let workload = RenderWorkload::new(engine(256 * MIB, Duration::from_secs(30)));
    let specs = workload.split(&input, 2).expect("découpe");
    let output = workload.execute(&input, &specs[0]).expect("exécution");
    assert_eq!(output.result.len(), 16 * 16 * 3);
    assert_eq!(output.milestones.len(), 2);
    let verdict = workload
        .verify(&input, &specs[0], &output, &[0])
        .expect("vérification");
    assert_eq!(verdict, Verdict::Approve, "un rendu honnête est approuvé");
}

#[test]
fn a_slow_wasm_engine_is_interrupted_at_the_deadline() {
    let renderer =
        engine(256 * MIB, Duration::from_millis(300)).with_extra_args(vec!["slow".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("délai dépassé");
    assert!(
        matches!(&error, WorkloadError::Execution(text) if text.contains("délai")),
        "{error:?}"
    );
}

#[test]
fn a_wasm_engine_with_runaway_output_is_cut_off() {
    let renderer = engine(256 * MIB, Duration::from_secs(30)).with_extra_args(vec!["flood".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("sortie trop grande");
    assert!(matches!(error, WorkloadError::OverLimit(_)), "{error:?}");
}

#[test]
fn a_wasm_engine_that_returns_too_few_bytes_is_refused() {
    let renderer = engine(256 * MIB, Duration::from_secs(30)).with_extra_args(vec!["short".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("octets manquants");
    assert!(matches!(&error, WorkloadError::Execution(text) if text.contains("octets rendus")));
}

#[test]
fn a_wasm_engine_has_no_network() {
    let renderer = engine(256 * MIB, Duration::from_secs(30)).with_extra_args(vec!["net".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("réseau refusé");
    assert!(
        matches!(&error, WorkloadError::Execution(text) if text.contains("code 10")),
        "{error:?}"
    );
}

#[test]
fn a_wasm_engine_cannot_write_to_the_filesystem() {
    let renderer = engine(256 * MIB, Duration::from_secs(30)).with_extra_args(vec!["write".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("écriture refusée");
    assert!(
        matches!(&error, WorkloadError::Execution(text) if text.contains("code 11")),
        "{error:?}"
    );
}

#[test]
fn a_wasm_engine_cannot_allocate_beyond_its_memory_limit() {
    let renderer = engine(64 * MIB, Duration::from_secs(30)).with_extra_args(vec!["alloc".into()]);
    let error = renderer
        .render(&scene(), 16, 24, 0, 8)
        .expect_err("allocation refusée");
    assert!(
        matches!(&error, WorkloadError::Execution(text) if text.contains("code 13")),
        "{error:?}"
    );
}
