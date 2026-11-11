//! Test de référence : le moteur Rust doit reproduire les vecteurs mesurés en Python (même modèle
//! fp32, même ONNX Runtime 1.31.1). Ignoré par défaut : il demande le runtime, le modèle et le
//! tokenizer, passés par variables d'environnement.
//!
//!   SIL_EMBED_RUNTIME    chemin de onnxruntime.dll / libonnxruntime.so (1.31.1)
//!   SIL_EMBED_MODEL      chemin de model.onnx (fp32)
//!   SIL_EMBED_TOKENIZER  chemin de tokenizer.json
//!
//! Lancer : cargo test -p sil-wl-embed --test reference -- --ignored

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_wl_embed::{DIM, Engine};
use std::path::PathBuf;

const TEXT_0: &str = "Le calcul distribué permet de répartir un travail entre plusieurs machines.";
// Huit premières composantes du vecteur de TEXT_0, mesurées en Python (ONNX Runtime 1.31.1).
const REFERENCE_0: [f32; 8] = [
    0.015_24,
    -0.011_865,
    -0.064_274_5,
    -0.064_986_6,
    0.093_888_8,
    -0.017_359_76,
    0.023_758_96,
    0.060_226_72,
];

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("variable {name} absente")))
}

#[test]
#[ignore = "demande ONNX Runtime, le modèle et le tokenizer (voir l'en-tête)"]
fn the_engine_reproduces_the_measured_vectors() {
    let mut engine = Engine::open(
        &env_path("SIL_EMBED_RUNTIME"),
        &env_path("SIL_EMBED_MODEL"),
        &env_path("SIL_EMBED_TOKENIZER"),
    )
    .unwrap();
    let first = engine.embed(TEXT_0).unwrap();
    assert_eq!(first.len(), DIM);
    let norm: f32 = first.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-5, "norme {norm}");
    for (got, want) in first.iter().zip(REFERENCE_0) {
        assert!((got - want).abs() < 1e-5, "{got} != {want}");
    }
    // Déterminisme : même texte, même vecteur, bit à bit.
    assert_eq!(engine.embed(TEXT_0).unwrap(), first);
}
