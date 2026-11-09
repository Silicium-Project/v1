use crate::{
    NetError, NetEvent, NetLimits, NetworkHandle, Providers, Topic, behaviour, clock::Clock,
    driver::Driver, ratelimit::PeerLimiters,
};
use libp2p::{Multiaddr, identity::Keypair};
use std::collections::{BTreeSet, HashMap};
use tokio::{sync::mpsc, task::JoinHandle};

/// Capacité des files entre l'application et la boucle réseau.
const QUEUE: usize = 1024;

#[derive(Debug, Clone, Default)]
pub struct NetConfig {
    pub listen: Vec<Multiaddr>,
    /// Pairs d'amorçage (adresses épinglées `/p2p/<id>`), recontactés tant qu'ils manquent.
    pub bootstrap: Vec<Multiaddr>,
    pub limits: NetLimits,
    /// Topics auxquels s'abonner au démarrage.
    pub subscribe: Vec<Topic>,
    /// Ce nœud sert de relais public (il doit être joignable directement).
    pub relay_server: bool,
}

/// Réseau démarré : poignée, flux d'événements, tâche de la boucle.
pub struct Network {
    pub handle: NetworkHandle,
    pub events: mpsc::Receiver<NetEvent>,
    pub task: JoinHandle<()>,
}

/// Démarre le réseau. À appeler depuis un runtime tokio.
pub fn start(
    keypair: Keypair,
    config: NetConfig,
    providers: Providers,
) -> Result<Network, NetError> {
    let mut swarm = behaviour::build(keypair, &config.limits, config.relay_server)?;
    for topic in &config.subscribe {
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic.ident())
            .map_err(|e| NetError::Config(e.to_string()))?;
    }
    for address in &config.listen {
        swarm
            .listen_on(address.clone())
            .map_err(|e| NetError::Listen(e.to_string()))?;
    }
    for address in &config.bootstrap {
        let _ = swarm.dial(address.clone());
    }
    let local = *swarm.local_peer_id();
    let (command_tx, commands) = mpsc::channel(QUEUE);
    let (events_tx, events) = mpsc::channel(QUEUE);
    let driver = Driver {
        swarm,
        commands,
        events: events_tx,
        providers,
        limiters: PeerLimiters::new(&config.limits),
        limits: config.limits.clone(),
        clock: Clock::new(),
        listen_addrs: Vec::new(),
        bootstrap: config.bootstrap,
        pending_ledger: HashMap::new(),
        pending_content: HashMap::new(),
        pending_providers: HashMap::new(),
        relay_listeners: HashMap::new(),
        reserved_relays: BTreeSet::new(),
    };
    let task = tokio::spawn(driver.run());
    Ok(Network {
        handle: NetworkHandle {
            commands: command_tx,
            local,
            limits: config.limits,
        },
        events,
        task,
    })
}
