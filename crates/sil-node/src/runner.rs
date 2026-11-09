//! Démarrage et arrêt du nœud.

use crate::{
    NodeBackend, NodeError, NodeOptions, Roles, finalize_loop, gossip_loop,
    providers::NodeLedgerProvider,
    settlement_loop,
    shared::{Shared, WorkBook},
    validator::NodeValidator,
    verifier_loop, worker_loop,
};
use sil_content::{ContentStore, StoreConfig as ContentConfig};
use sil_control_api::{ApiLimits, TokenSet, router, serve};
use sil_crypto::{MasterSeed, Purpose};
use sil_ledger::Ledger;
use sil_ledger_store::{EventStore, StoreConfig as LogConfig};
use sil_net::{NetConfig, NetworkHandle, Providers, Topic, keypair_from_secret};
use sil_reputation::{Reputation, ReputationConfig};
use sil_settlement::{Chain, OutboxConfig, OutboxFile};
use sil_transfer::StoreProvider;
use sil_workload_api::Registry;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio::{sync::watch, task::JoinHandle};

/// Nœud démarré.
pub struct Node {
    shared: Arc<Shared>,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    net: NetworkHandle,
    control_addr: Option<SocketAddr>,
}

fn storage(error: impl ToString) -> NodeError {
    NodeError::Storage(error.to_string())
}

impl Node {
    /// Démarre le nœud : rejoue le journal, ouvre les magasins, lance réseau, acteurs et API.
    pub async fn start(
        options: NodeOptions,
        seed: MasterSeed,
        registry: Registry,
        chain: Arc<dyn Chain>,
    ) -> Result<Self, NodeError> {
        let dir = options.data_dir.clone();
        let mut ledger = Ledger::open_to_all(options.ledger_limits.clone());
        let (log, report) = EventStore::open(LogConfig::new(dir.join("ledger")), |event| {
            let _ = ledger.receive(event);
        })
        .map_err(storage)?;
        tracing::info!(events = report.events_read, "journal rejoué");
        let store = ContentStore::open(ContentConfig::new(dir.join("content"))).map_err(storage)?;
        let outbox_file = OutboxFile::new(dir.join("settlement").join("outbox.cbor"));
        let outbox = outbox_file.load(OutboxConfig::default()).map_err(storage)?;
        let identity = seed.derive(Purpose::Ledger);
        let keypair = keypair_from_secret(&seed.derive_raw(Purpose::Network))
            .map_err(|e| NodeError::Net(e.to_string()))?;

        let shared = Arc::new(Shared {
            ledger: Mutex::new(ledger),
            log: Mutex::new(log),
            store: store.clone(),
            registry,
            reputation: Mutex::new(Reputation::new(
                ReputationConfig::default(),
                options.trusted_requesters.clone(),
            )),
            verify_reputation: Mutex::new(Reputation::new(
                ReputationConfig::default(),
                options.trusted_requesters.clone(),
            )),
            derived: Mutex::new(Default::default()),
            outbox: Mutex::new(outbox),
            outbox_file,
            chain,
            seed,
            identity,
            roles: options.roles,
            free_work: options.free_work,
            net: OnceLock::new(),
            work: Mutex::new(WorkBook::default()),
            runtime: tokio::runtime::Handle::current(),
            active: std::sync::atomic::AtomicU32::new(0),
        });

        let providers = Providers {
            ledger: Arc::new(NodeLedgerProvider(Arc::clone(&shared))),
            content: Arc::new(StoreProvider::new(store)),
            validator: Arc::new(NodeValidator),
        };
        let network = sil_net::start(
            keypair,
            NetConfig {
                listen: options.listen.clone(),
                bootstrap: options.bootstrap.clone(),
                relay_server: options.relay_server,
                limits: options.net_limits.clone(),
                // Présence et annonces seulement : un fragment de jobs s'ajoute quand un job
                // de ce fragment est connu (`Shared::follow_job`).
                subscribe: vec![Topic::Presence, Topic::Announce],
            },
            providers,
        )
        .map_err(|e| NodeError::Net(e.to_string()))?;
        let net = network.handle.clone();
        let _ = shared.net.set(net.clone());
        for job in shared.job_ids() {
            shared.follow_job(&job);
        }

        let (stop, stop_rx) = watch::channel(false);
        let tick = Duration::from_millis(options.tick_ms.max(10));
        let mut tasks = vec![
            tokio::spawn(gossip_loop::run(
                Arc::clone(&shared),
                network.events,
                stop_rx.clone(),
            )),
            tokio::spawn(settlement_loop::run(Arc::clone(&shared), stop_rx.clone())),
            tokio::spawn(crate::presence_loop::run(
                Arc::clone(&shared),
                stop_rx.clone(),
            )),
        ];
        let Roles { worker, verifier } = options.roles;
        // Finalisation : tout nœud (ADR 0005, phase 4 ; ADR 0007).
        tasks.push(tokio::spawn(finalize_loop::run(
            Arc::clone(&shared),
            tick,
            stop_rx.clone(),
        )));
        if worker {
            tasks.push(tokio::spawn(worker_loop::run(
                Arc::clone(&shared),
                tick,
                stop_rx.clone(),
            )));
        }
        if verifier {
            tasks.push(tokio::spawn(verifier_loop::run(
                Arc::clone(&shared),
                tick,
                stop_rx.clone(),
            )));
        }

        let mut control_addr = None;
        if let Some(control) = &options.control {
            let tokens = TokenSet::load_or_create(&control.token_file)
                .map_err(|e| NodeError::Control(e.to_string()))?;
            let app = router(
                Arc::new(NodeBackend::new(Arc::clone(&shared))),
                tokens,
                ApiLimits::default(),
            );
            let listener = tokio::net::TcpListener::bind(control.bind)
                .await
                .map_err(|e| NodeError::Control(e.to_string()))?;
            control_addr = listener.local_addr().ok();
            let mut stopped = stop_rx.clone();
            tasks.push(tokio::spawn(async move {
                let shutdown = async move {
                    let _ = stopped.changed().await;
                };
                if let Err(error) = serve(listener, app, shutdown).await {
                    tracing::error!(%error, "API de contrôle arrêtée");
                }
            }));
        }

        Ok(Self {
            shared,
            stop,
            tasks,
            net,
            control_addr,
        })
    }

    pub fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    pub fn net(&self) -> &NetworkHandle {
        &self.net
    }

    /// Adresse réelle de l'API de contrôle, si elle est active.
    pub fn control_addr(&self) -> Option<SocketAddr> {
        self.control_addr
    }

    /// Arrête proprement : acteurs, API, réseau, puis synchronise le journal sur disque.
    pub async fn shutdown(self) {
        let _ = self.stop.send(true);
        self.net.shutdown().await;
        for task in self.tasks {
            if tokio::time::timeout(Duration::from_secs(5), task)
                .await
                .is_err()
            {
                tracing::warn!("un acteur ne s'est pas arrêté à temps");
            }
        }
        if let Err(error) = crate::shared::lock(&self.shared.log).sync() {
            tracing::error!(%error, "journal non synchronisé à l'arrêt");
        }
    }
}
