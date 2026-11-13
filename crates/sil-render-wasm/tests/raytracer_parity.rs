//! Le raytracer compilé en WebAssembly rend exactement les mêmes octets que le même code natif.
//! Sinon, un vérificateur natif refuserait les bandes rendues en WASM.
//!
//! Moteur attendu : `cargo build -p sil-raytracer --bin sil-raytracer-block --target wasm32-wasip1 --release`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use sil_raytracer::render::render_rows;
use sil_render_wasm::WasmRenderer;
use sil_wl_render::BlockRenderer;
use std::{path::Path, time::Duration};

const MIB: usize = 1024 * 1024;
const SCENE: &str = "render : { samples = 1 }
camera : { position = (0, 0, -5) look_at = (0, 0, 0) fov = 40 resolution = { width = 96 height = 64 } }
sphere : { center = (0, 0, 0) radius = 1 material = { colour = (0.8, 0.2, 0.2) } }
sphere : { center = (1.5, 0.5, 1) radius = 0.6 material = { colour = (0.2, 0.7, 0.3) } }
lights : { point = { x = 2 y = 3 z = 4 intensity = 450 } }
";

fn engine_path() -> std::path::PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-wasip1/release/sil-raytracer-block.wasm");
    assert!(path.exists(), "moteur absent : cargo build -p sil-raytracer --bin sil-raytracer-block --target wasm32-wasip1 --release");
    path
}

#[test]
fn the_raytracer_on_wasm_renders_exactly_the_native_pixels() {
    let wasm = WasmRenderer::new(&engine_path(), 512 * MIB, Duration::from_secs(120)).expect("moteur");
    let scene = SCENE.as_bytes();
    let (width, height) = (96, 64);
    for (y0, y1) in [(0, 16), (16, 40), (40, 64)] {
        let on_wasm = wasm.render(scene, width, height, y0, y1).expect("rendu wasm");
        let native = render_rows(SCENE, width, height, y0, y1).expect("rendu natif");
        assert_eq!(on_wasm, native, "bande {y0}..{y1} différente");
    }
}

/// Mesure : temps de rendu d'une bande 800 x 150 px, natif (multi-cœurs) contre WASM (un cœur).
/// Lancer : `cargo test -p sil-render-wasm --release --test raytracer_parity -- --ignored --nocapture`.
#[test]
#[ignore = "mesure de vitesse"]
fn measure_band_time_native_and_wasm() {
    let scene = SCENE.replace("width = 96 height = 64", "width = 4600 height = 4600").replace("samples = 1", "samples = 8");
    let (width, height, y0, y1) = (4600, 4600, 2000, 2288);
    let wasm = WasmRenderer::new(&engine_path(), 512 * MIB, Duration::from_secs(600)).expect("moteur");
    let start = std::time::Instant::now();
    let native = render_rows(&scene, width, height, y0, y1).expect("rendu natif");
    let native_time = start.elapsed();
    let start = std::time::Instant::now();
    let on_wasm = wasm.render(scene.as_bytes(), width, height, y0, y1).expect("rendu wasm");
    let wasm_time = start.elapsed();
    assert_eq!(on_wasm, native, "bandes différentes");
    println!("natif : {:.2} s · wasm : {:.2} s · ratio : {:.1}x", native_time.as_secs_f64(), wasm_time.as_secs_f64(), wasm_time.as_secs_f64() / native_time.as_secs_f64());
}
