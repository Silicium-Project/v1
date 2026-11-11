//! Workload de bout en bout : découpage, calcul, verdict par tolérance. Ignoré par défaut
//! (demande ONNX Runtime, le modèle et le tokenizer, voir tests/reference.rs).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_wl_embed::{EmbedWorkload, Engine, decode_vector};
use sil_workload_api::{MemoryInput, Workload};
use std::path::PathBuf;

const TEXT: &str = "Le calcul distribué permet de répartir un travail.\nThe cat sleeps on the sofa.\nFacture 2026-0412 : 1 248,50 euros.\nSELECT id FROM jobs;";

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("variable {name} absente")))
}

fn workload() -> EmbedWorkload {
    EmbedWorkload::new(
        Engine::open(
            &env_path("SIL_EMBED_RUNTIME"),
            &env_path("SIL_EMBED_MODEL"),
            &env_path("SIL_EMBED_TOKENIZER"),
        )
        .unwrap(),
    )
}

#[test]
#[ignore = "demande ONNX Runtime, le modèle et le tokenizer"]
fn an_honest_result_passes_the_tolerance_check() {
    let bytes = TEXT.as_bytes().to_vec();
    let input = MemoryInput(&bytes);
    let work = workload();
    let specs = work.split(&input, 2).unwrap();
    assert_eq!(specs.len(), 2);
    let spec = &specs[0];
    let output = work.execute(&input, spec).unwrap();
    let blocks = work.block_count(spec).unwrap();
    let sample: Vec<u32> = (0..blocks).collect();
    let judged = work
        .verify_tolerance(&input, spec, &output, &sample)
        .unwrap();
    assert!(
        judged.min_cos_micro >= 999_000,
        "cosinus {}",
        judged.min_cos_micro
    );
}

#[test]
#[ignore = "demande ONNX Runtime, le modèle et le tokenizer"]
fn a_tampered_vector_fails_the_tolerance_check() {
    let bytes = TEXT.as_bytes().to_vec();
    let input = MemoryInput(&bytes);
    let work = workload();
    let specs = work.split(&input, 1).unwrap();
    let spec = &specs[0];
    let mut output = work.execute(&input, spec).unwrap();
    // Un vecteur remplacé par un autre texte : cosinus loin du seuil.
    let other = work.execute(&input, &specs[0]).unwrap();
    let blocks = work.block_count(spec).unwrap();
    assert!(blocks >= 2);
    let mut vector = decode_vector(&other.result, 1).unwrap();
    vector.reverse();
    let bytes_of: Vec<u8> = vector.iter().flat_map(|v| v.to_le_bytes()).collect();
    let start = 384 * 4;
    output.result[start..start + 384 * 4].copy_from_slice(&bytes_of);
    let sample = vec![1u32];
    let judged = work
        .verify_tolerance(&input, spec, &output, &sample)
        .unwrap();
    assert!(
        judged.min_cos_micro < 999_000,
        "cosinus {}",
        judged.min_cos_micro
    );
}
