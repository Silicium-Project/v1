//! Moteur de rendu du flux complet. `SIL_RENDER_ENGINE` désigne :
//! - un module `.wasm` : moteur WebAssembly, isolé par construction (même sur Windows) ;
//! - un programme natif : moteur lancé dans `sil-sandbox` (`SIL_RENDER_ISOLATION` : `strict`
//!   par défaut, `best-effort` si l'isolation complète n'existe pas sur la plateforme).
//!
//! Voir `chain/scripts/localnet.sh` pour la construction du moteur de référence.

use sil_render_wasm::WasmRenderer;
use sil_sandbox::{Isolation, Level, default_launcher};
use sil_wl_render::{BlockRenderer, ExternalRenderer};
use sil_workload_api::WorkloadError;
use std::{path::PathBuf, time::Duration};

/// Mémoire maximale d'une instance WebAssembly du flux complet.
const WASM_MEMORY_BYTES: usize = 256 * 1024 * 1024;

/// Moteur choisi par `SIL_RENDER_ENGINE`.
pub enum Engine {
    Native(ExternalRenderer),
    Wasm(WasmRenderer),
}

impl BlockRenderer for Engine {
    fn render(
        &self,
        scene: &[u8],
        width: u32,
        height: u32,
        y0: u32,
        y1: u32,
    ) -> Result<Vec<u8>, WorkloadError> {
        match self {
            Engine::Native(renderer) => renderer.render(scene, width, height, y0, y1),
            Engine::Wasm(renderer) => renderer.render(scene, width, height, y0, y1),
        }
    }
}

pub fn engine() -> Engine {
    let program = PathBuf::from(
        std::env::var("SIL_RENDER_ENGINE").expect("SIL_RENDER_ENGINE (voir chain/README.md)"),
    );
    if program.extension().is_some_and(|ext| ext == "wasm") {
        let renderer = WasmRenderer::new(&program, WASM_MEMORY_BYTES, Duration::from_secs(60))
            .expect("moteur WebAssembly chargé");
        return Engine::Wasm(renderer);
    }
    let work = std::env::temp_dir().join("sil-e2e-render");
    std::fs::create_dir_all(&work).expect("dossier de rendu");
    let mut isolation = Isolation::for_program(&program, &work, default_launcher());
    isolation.level = level();
    Engine::Native(
        ExternalRenderer::new(program, work, Duration::from_secs(60)).with_isolation(isolation),
    )
}

fn level() -> Level {
    match std::env::var("SIL_RENDER_ISOLATION").as_deref() {
        Ok("best-effort") => Level::BestEffort,
        _ => Level::Strict,
    }
}
