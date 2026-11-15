//! Moteur de rendu en WebAssembly (WASI preview 1), même protocole que `sil-wl-render::ExternalRenderer`.
//!
//! Isolation **par construction**, identique sur Windows, Linux et macOS :
//! - pas de réseau : WASI preview 1 n'a pas de sockets ;
//! - pas de fichiers : aucun dossier n'est pré-ouvert ;
//! - pas d'environnement : rien n'est transmis, ni variable ni argument hérité ;
//! - mémoire bornée (`memory_bytes`) ;
//! - délai par bloc (`wall`), appliqué par epoch : le moteur est interrompu au-delà ;
//! - sortie bornée : exactement la bande attendue, tout octet de plus fait échouer le moteur.
//!
//! Le module est compilé une seule fois à la construction ; chaque bloc reçoit une instance neuve.

use sil_wl_render::BlockRenderer;
use sil_workload_api::WorkloadError;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use wasmtime::{
    Config, Engine, InstancePre, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Trap,
};
use wasmtime_wasi::{
    I32Exit, WasiCtxBuilder,
    p1::{self, WasiP1Ctx},
    p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
};

/// Période de l'horloge epoch : la précision du délai.
const EPOCH_TICK: Duration = Duration::from_millis(10);
/// Plafond de la sortie d'erreur du moteur.
const MAX_STDERR: usize = 64 * 1024;

/// État d'un appel : contexte WASI et limites de mémoire.
struct Host {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
}

pub struct WasmRenderer {
    engine: Engine,
    pre: InstancePre<Host>,
    memory_bytes: usize,
    wall: Duration,
    stop: Arc<AtomicBool>,
    extra_args: Vec<String>,
    /// Nombre maximal de sous-bandes rendues en parallèle pour un bloc.
    threads: usize,
}

impl WasmRenderer {
    /// Charge le module `wasm` (commande WASI preview 1 : `_start`). `memory_bytes` : mémoire
    /// maximale d'une instance. `wall` : durée maximale d'un bloc.
    pub fn new(wasm: &Path, memory_bytes: usize, wall: Duration) -> Result<Self, String> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|e| format!("moteur wasm : {e}"))?;
        let module = Module::from_file(&engine, wasm)
            .map_err(|e| format!("module wasm illisible {} : {e}", wasm.display()))?;
        let mut linker: Linker<Host> = Linker::new(&engine);
        p1::add_to_linker_sync(&mut linker, |host: &mut Host| &mut host.wasi)
            .map_err(|e| format!("liaison WASI : {e}"))?;
        let pre = linker
            .instantiate_pre(&module)
            .map_err(|e| format!("module wasm non instanciable : {e}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        spawn_ticker(engine.clone(), Arc::clone(&stop));
        Ok(Self {
            engine,
            pre,
            memory_bytes,
            wall,
            stop,
            extra_args: Vec::new(),
            threads: thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        })
    }

    /// Nombre maximal de sous-bandes rendues en parallèle (au moins une). Ne change pas les octets.
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads.max(1);
        self
    }

    /// Arguments ajoutés après les quatre du protocole. Réservé aux tests (modes de panne du
    /// moteur de référence) : la configuration du nœud ne les expose pas.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }
}

/// Fait avancer l'horloge epoch tant que le moteur existe. Le délai de chaque appel est relatif
/// à cette horloge, donc plusieurs appels simultanés sur le même moteur restent indépendants.
fn spawn_ticker(engine: Engine, stop: Arc<AtomicBool>) {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            thread::sleep(EPOCH_TICK);
            engine.increment_epoch();
        }
    });
}

impl Drop for WasmRenderer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl WasmRenderer {
    /// Rend `[y0, y1)` dans une instance neuve, sur un seul thread.
    fn render_part(
        &self,
        scene: &[u8],
        width: u32,
        height: u32,
        y0: u32,
        y1: u32,
    ) -> Result<Vec<u8>, WorkloadError> {
        let expected = usize::try_from(u64::from(width) * u64::from(y1 - y0) * 3)
            .map_err(|_| WorkloadError::OverLimit("taille du bloc"))?;
        let mut args = vec![
            "render".to_string(),
            width.to_string(),
            height.to_string(),
            y0.to_string(),
            y1.to_string(),
        ];
        args.extend(self.extra_args.iter().cloned());
        let stdout = MemoryOutputPipe::new(expected);
        let stderr = MemoryOutputPipe::new(MAX_STDERR);
        let wasi = WasiCtxBuilder::new()
            .args(&args)
            .stdin(MemoryInputPipe::new(scene.to_vec()))
            .stdout(stdout.clone())
            .stderr(stderr.clone())
            .build_p1();
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_bytes)
            .build();
        let mut store = Store::new(&self.engine, Host { wasi, limits });
        store.limiter(|host| &mut host.limits);
        store.set_epoch_deadline(ticks(self.wall));

        let instance = self
            .pre
            .instantiate(&mut store)
            .map_err(|e| WorkloadError::Execution(format!("moteur : instanciation : {e}")))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| WorkloadError::Execution(format!("moteur : pas de _start : {e}")))?;
        // Une sortie `proc_exit(0)` (fin normale de `main` en Rust) est un succès.
        if let Err(error) = start.call(&mut store, ())
            && error.downcast_ref::<I32Exit>().map(|exit| exit.0) != Some(0)
        {
            return Err(classify(&error, &stderr));
        }
        let rendered = stdout.contents();
        if rendered.len() != expected {
            return Err(WorkloadError::Execution(format!(
                "moteur : {} octets rendus, {expected} attendus",
                rendered.len()
            )));
        }
        Ok(rendered.to_vec())
    }
}

impl BlockRenderer for WasmRenderer {
    /// Les lignes sont indépendantes : la bande est découpée en sous-bandes contiguës, rendues en
    /// parallèle (une instance par sous-bande), puis concaténées dans l'ordre. Les octets ne
    /// dépendent pas du nombre de sous-bandes (même propriété que le moteur natif).
    fn render(
        &self,
        scene: &[u8],
        width: u32,
        height: u32,
        y0: u32,
        y1: u32,
    ) -> Result<Vec<u8>, WorkloadError> {
        let rows = y1.saturating_sub(y0);
        let parts = self.threads.min(rows as usize).max(1);
        if parts == 1 {
            return self.render_part(scene, width, height, y0, y1);
        }
        let bounds: Vec<(u32, u32)> = (0..parts)
            .map(|i| {
                let at = |k: usize| y0 + (u64::from(rows) * k as u64 / parts as u64) as u32;
                (at(i), at(i + 1))
            })
            .collect();
        let results: Vec<Result<Vec<u8>, WorkloadError>> = thread::scope(|scope| {
            let handles: Vec<_> = bounds
                .iter()
                .map(|&(a, b)| scope.spawn(move || self.render_part(scene, width, height, a, b)))
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .unwrap_or_else(|_| Err(WorkloadError::Execution("moteur : thread".into())))
                })
                .collect()
        });
        let mut rendered = Vec::new();
        for part in results {
            rendered.extend(part?);
        }
        Ok(rendered)
    }
}

/// Nombre d'horloges epoch correspondant à une durée (au moins une).
fn ticks(wall: Duration) -> u64 {
    let tick = EPOCH_TICK.as_millis().max(1);
    u64::try_from(wall.as_millis().div_ceil(tick).max(1)).unwrap_or(u64::MAX)
}

/// Code de sortie par lequel le moteur signale que l'hôte a refusé sa sortie (protocole).
const CODE_SORTIE_REFUSEE: i32 = 12;

/// Traduit l'arrêt d'un moteur : délai (interruption epoch), sortie refusée, autre code de sortie,
/// autre trap.
fn classify(error: &wasmtime::Error, stderr: &MemoryOutputPipe) -> WorkloadError {
    if matches!(error.downcast_ref::<Trap>(), Some(Trap::Interrupt)) {
        return WorkloadError::Execution("moteur : délai dépassé".into());
    }
    if let Some(exit) = error.downcast_ref::<I32Exit>() {
        if exit.0 == CODE_SORTIE_REFUSEE {
            return WorkloadError::OverLimit("sortie du moteur trop grande");
        }
        return WorkloadError::Execution(format!(
            "moteur : code {} {}",
            exit.0,
            String::from_utf8_lossy(&stderr.contents())
                .chars()
                .take(200)
                .collect::<String>()
        ));
    }
    let first_line = error
        .to_string()
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    WorkloadError::Execution(format!("moteur : {first_line}"))
}
