//! Démarrage du nœud en ligne de commande : configuration, clé, options, chaîne, arrêt propre.
//!
//! La chaîne n'est pas choisie ici : l'appelant fournit une fabrique (`ChainFactory`). Le binaire
//! `silicium-node` de `sil-node` n'a pas de chaîne ; `chain/node` fournit la chaîne Solana.

use crate::{ControlOptions, Node, NodeOptions, Roles};
use sil_chain_core::Escrow;
use sil_config::{ChainConfig, IsolationMode, NodeConfig};
use sil_crypto::{Purpose, keyfile};
use sil_net::Multiaddr;
use sil_render_wasm::WasmRenderer;
use sil_sandbox::{Isolation, Level};

/// Mémoire maximale d'une instance de moteur WebAssembly.
const WASM_MEMORY_BYTES: usize = 256 * 1024 * 1024;
use sil_settlement::{Authorization, Chain, ChainError, TxId, TxStatus};
use sil_telemetry::TelemetryConfig;
use sil_types::{JobId, PublicKey};
use sil_wl_render::{ExternalRenderer, RenderWorkload};
use sil_workload_api::Registry;
use std::{net::SocketAddr, path::PathBuf, process::ExitCode, sync::Arc, time::Duration};

/// Construit la chaîne à partir de la section `[chain]` de la configuration.
pub type ChainFactory = fn(&ChainConfig) -> Result<Arc<dyn Chain>, String>;

/// Chaîne absente : toute opération échoue, visiblement.
pub struct NoChain;

impl Chain for NoChain {
    fn escrow(&self, _: &JobId) -> Result<Option<Escrow>, ChainError> {
        Err(ChainError::Transient("aucune chaîne configurée".into()))
    }

    fn submit(&self, _: &Authorization) -> Result<TxId, ChainError> {
        Err(ChainError::Transient("aucune chaîne configurée".into()))
    }

    fn tx_status(&self, _: &TxId) -> Result<TxStatus, ChainError> {
        Err(ChainError::Transient("aucune chaîne configurée".into()))
    }
}

/// Fabrique du binaire `sil-node` : pas de chaîne. Une configuration qui en demande une est refusée.
pub fn without_chain(_: &ChainConfig) -> Result<Arc<dyn Chain>, String> {
    Err("la configuration demande une chaîne Solana : utilisez le binaire chain/node (silicium-node avec Solana)".into())
}

struct Args {
    config: Option<PathBuf>,
    roles: Roles,
    public_key: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        config: None,
        roles: Roles::default(),
        public_key: false,
    };
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        if flag == "--public-key" {
            args.public_key = true;
            continue;
        }
        let value = iter.next().ok_or(format!("{flag} attend une valeur"))?;
        match flag.as_str() {
            "--config" => args.config = Some(value.into()),
            "--roles" => {
                for role in value.split(',') {
                    match role.trim() {
                        "worker" => args.roles.worker = true,
                        "verifier" => args.roles.verifier = true,
                        other => return Err(format!("rôle inconnu : {other}")),
                    }
                }
            }
            other => return Err(format!("option inconnue : {other}")),
        }
    }
    Ok(args)
}

fn addresses(list: &[String]) -> Result<Vec<Multiaddr>, String> {
    list.iter()
        .map(|a| a.parse().map_err(|e| format!("adresse {a} : {e}")))
        .collect()
}

fn options(config: &NodeConfig, roles: Roles) -> Result<NodeOptions, String> {
    let mut options = NodeOptions::new(&config.data_dir);
    options.roles = roles;
    options.free_work = config.testnet.free_work;
    options.listen = addresses(&config.network.listen)?;
    options.bootstrap = addresses(&config.network.bootstrap)?;
    options.relay_server = config.network.relay_server;
    options.control = Some(ControlOptions {
        bind: SocketAddr::new(config.control.bind, config.control.port),
        token_file: config
            .control
            .token_file
            .clone()
            .unwrap_or_else(|| config.data_dir.join("control.token")),
    });
    options.trusted_requesters = config
        .trust
        .requesters
        .iter()
        .map(|hex| PublicKey::from_hex(hex).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    Ok(options)
}

fn chain_from_config(config: &NodeConfig, factory: ChainFactory) -> Result<Arc<dyn Chain>, String> {
    match &config.chain {
        Some(chain) => factory(chain),
        None => {
            tracing::warn!("aucune chaîne configurée : les workers ne réclameront aucun fragment");
            Ok(Arc::new(NoChain))
        }
    }
}

/// Politique d'isolation du moteur de rendu selon `[render] isolation`. `None` : `off`.
/// `strict` échoue au démarrage si l'isolation complète ne peut pas être appliquée ici.
fn render_isolation(
    render: &sil_config::RenderConfig,
    work: &std::path::Path,
) -> Result<Option<Isolation>, String> {
    let level = match render.isolation {
        IsolationMode::Off => return Ok(None),
        IsolationMode::BestEffort => Level::BestEffort,
        IsolationMode::Strict => Level::Strict,
    };
    if level == Level::Strict && !cfg!(target_os = "linux") {
        return Err(
            "isolation stricte indisponible sous ce système : mettre [render] isolation = \"best-effort\" ou \"off\""
                .into(),
        );
    }
    let launcher = sil_sandbox::default_launcher();
    if cfg!(target_os = "linux") && launcher.is_none() {
        return Err(
            "lanceur sil-sandbox-launch introuvable : le placer à côté de silicium-node ou définir SIL_SANDBOX_LAUNCHER"
                .into(),
        );
    }
    let mut policy = Isolation::for_program(&render.engine, work, launcher);
    policy.level = level;
    Ok(Some(policy))
}

/// Workloads servis par ce nœud : sha256 (dans le processus, code de confiance) et, si `[render]`
/// est configuré, le rendu par un moteur externe lancé dans `sil-sandbox`.
fn registry_for(config: &NodeConfig) -> Result<Registry, String> {
    let mut registry = Registry::new().with(Arc::new(sil_wl_sha256::Sha256Workload));
    if let Some(render) = &config.render {
        if !render.engine.is_file() {
            return Err(format!(
                "moteur de rendu introuvable : {}",
                render.engine.display()
            ));
        }
        // Un moteur `.wasm` est isolé par construction (WebAssembly) : même comportement sur tous
        // les systèmes, sans réglage d'isolation.
        if render
            .engine
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("wasm"))
        {
            let engine = WasmRenderer::new(
                &render.engine,
                WASM_MEMORY_BYTES,
                Duration::from_millis(render.wall_ms),
            )?;
            return Ok(registry.with(Arc::new(RenderWorkload::new(engine))));
        }
        // Le moteur natif n'écrit que dans son propre dossier : jamais dans `data_dir`, qui contient
        // les clés, et il ne lit pas `data_dir` (voir `render_isolation`).
        let work = config.data_dir.join("render-work");
        std::fs::create_dir_all(&work).map_err(|e| format!("dossier de rendu : {e}"))?;
        let mut engine = ExternalRenderer::new(
            render.engine.clone(),
            work.clone(),
            Duration::from_millis(render.wall_ms),
        );
        if let Some(isolation) = render_isolation(render, &work)? {
            engine = engine.with_isolation(isolation);
        }
        registry = registry.with(Arc::new(RenderWorkload::new(engine)));
    }
    Ok(registry)
}

/// Affiche la clé publique du nœud (identité de signature, utile au diagnostic).
/// Ne crée jamais de graine : sans `node.seed`, c'est une erreur. Seule la clé publique sort.
fn print_public_key(config: &NodeConfig) -> Result<(), String> {
    let seed = keyfile::load(&config.data_dir.join("node.seed")).map_err(|e| e.to_string())?;
    println!("{}", seed.derive(Purpose::Ledger).public().to_hex());
    Ok(())
}

async fn run(factory: ChainFactory) -> Result<(), String> {
    let args = parse_args()?;
    let config = sil_config::load(args.config.as_deref(), &|key| std::env::var(key).ok())
        .map_err(|e| e.to_string())?;
    if args.public_key {
        return print_public_key(&config);
    }
    sil_telemetry::init(&TelemetryConfig {
        level: config.log.level.clone(),
        json: config.log.json,
    })
    .map_err(|e| e.to_string())?;
    if config.testnet.free_work {
        tracing::warn!("mode testnet : travail sans paiement, réseau de test seulement");
    }
    let seed =
        keyfile::load_or_create(&config.data_dir.join("node.seed")).map_err(|e| e.to_string())?;
    let registry = registry_for(&config)?;
    let chain = chain_from_config(&config, factory)?;
    let node = Node::start(options(&config, args.roles)?, seed, registry, chain)
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(peer = %node.net().local_peer_id(), api = ?node.control_addr(), "nœud démarré");
    tokio::signal::ctrl_c().await.map_err(|e| e.to_string())?;
    node.shutdown().await;
    Ok(())
}

/// Point d'entrée commun des binaires : lance le nœud avec la fabrique de chaîne donnée.
pub async fn main_with(factory: ChainFactory) -> ExitCode {
    match run(factory).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("erreur : {error}");
            ExitCode::FAILURE
        }
    }
}
